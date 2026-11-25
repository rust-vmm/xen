/*
 * Copyright 2022-23 Mathieu Poirier <mathieu.poirier@linaro.org>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

#![allow(clippy::type_complexity)]

use std::{
    collections::VecDeque,
    ffi::CString,
    fs::{File, OpenOptions},
    io::{Error, ErrorKind, Read, Write},
    mem,
    net::Shutdown,
    os::unix::{fs::OpenOptionsExt, io::AsRawFd, net::UnixStream},
    path::Path,
    sync::{Arc, Condvar, Mutex},
    thread,
    thread::JoinHandle,
};

use vmm_sys_util::eventfd::{EventFd, EFD_SEMAPHORE};
use xen_bindings::bindings::xs_watch_type;

use crate::types::*;

pub const XS_DIRECTORY: u32 = 1;
pub const XS_READ: u32 = 2;
pub const XS_WATCH: u32 = 4;
pub const XS_WRITE: u32 = 11;
pub const XS_WATCH_EVENT: u32 = 15;

fn message_bytes(message: &XenSocketMessage) -> &[u8] {
    // SAFETY: XenSocketMessage is #[repr(C)] with four u32 fields and no padding.
    unsafe {
        std::slice::from_raw_parts(
            std::ptr::addr_of!(*message).cast(),
            mem::size_of::<XenSocketMessage>(),
        )
    }
}

fn write_request<W: Write>(
    writer: &mut W,
    message: &XenSocketMessage,
    payload: &[u8],
) -> Result<(), std::io::Error> {
    writer.write_all(message_bytes(message))?;
    writer.write_all(payload)
}

enum XenStoreTransport {
    Socket(UnixStream),
    Device { file: File, stop_eventfd: EventFd },
}

fn default_transport(
    socket_path: &Path,
    device_path: &Path,
) -> Result<XenStoreTransport, std::io::Error> {
    UnixStream::connect(socket_path)
        .map(XenStoreTransport::Socket)
        .or_else(|_| {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(device_path)?;

            Ok(XenStoreTransport::Device {
                file,
                stop_eventfd: EventFd::new(0)?,
            })
        })
}

impl XenStoreTransport {
    fn try_clone(&self) -> Result<Self, std::io::Error> {
        match self {
            Self::Socket(stream) => stream.try_clone().map(Self::Socket),
            Self::Device { file, stop_eventfd } => Ok(Self::Device {
                file: file.try_clone()?,
                stop_eventfd: stop_eventfd.try_clone()?,
            }),
        }
    }

    fn shutdown(&self) {
        match self {
            Self::Socket(stream) => {
                /*
                 * Calling shutdown() on the socket will cause the blocking
                 * read in thread_function() to return with an error, causing
                 * the reader thread to stop.
                 */
                let _ = stream.shutdown(Shutdown::Both);
            }
            Self::Device { stop_eventfd, .. } => {
                // Socket shutdown cannot wake this reader; use an eventfd.
                let _ = stop_eventfd.write(1);
            }
        }
    }
}

impl Read for XenStoreTransport {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        match self {
            Self::Socket(stream) => stream.read(buffer),
            Self::Device { file, stop_eventfd } => {
                if buffer.is_empty() {
                    // Do not poll for a zero-length read.
                    return Ok(0);
                }

                // Poll for input or shutdown; repoll after transient read errors.
                loop {
                    if !wait_for_device_input(file, stop_eventfd)? {
                        return Ok(0);
                    }

                    match file.read(buffer) {
                        Err(error)
                            if error.kind() == ErrorKind::Interrupted
                                || error.kind() == ErrorKind::WouldBlock => {}
                        result => return result,
                    }
                }
            }
        }
    }
}

impl Write for XenStoreTransport {
    fn write(&mut self, buffer: &[u8]) -> Result<usize, std::io::Error> {
        match self {
            Self::Socket(stream) => stream.write(buffer),
            Self::Device { file, .. } => file.write(buffer),
        }
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        match self {
            Self::Socket(stream) => stream.flush(),
            Self::Device { file, .. } => file.flush(),
        }
    }
}

fn queue_message(
    condvar: &Arc<(
        Mutex<VecDeque<Result<XenStoreMessage, std::io::Error>>>,
        Condvar,
    )>,
    eventfd: Option<EventFd>,
    message: Result<XenStoreMessage, std::io::Error>,
) {
    let (lock, cvar) = &**condvar;

    let mut queue = lock.lock().unwrap();

    if let Some(eventfd) = eventfd {
        /* Increment evenfd counter to be consumed in read_watch() */
        let _ = eventfd
            .write(1)
            .map_err(|e| println!("queue_message: error: {}", e));
    }

    queue.push_back(message);
    cvar.notify_one();
}

fn wait_for_device_input(file: &File, stop_eventfd: &EventFd) -> Result<bool, std::io::Error> {
    let poll_fd = |fd| libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let mut poll_fds = [poll_fd(file.as_raw_fd()), poll_fd(stop_eventfd.as_raw_fd())];

    loop {
        // SAFETY: poll_fds is valid for both entries during the call.
        if unsafe { libc::poll(poll_fds.as_mut_ptr(), poll_fds.len() as _, -1) } < 0 {
            match Error::last_os_error() {
                error if error.kind() == ErrorKind::Interrupted => continue,
                error => return Err(error),
            }
        }

        // Give shutdown priority over pending input.
        if poll_fds[1].revents & libc::POLLIN != 0 {
            return Ok(false);
        }

        let revents = poll_fds[0].revents;
        if revents & libc::POLLNVAL != 0 {
            return Err(Error::from_raw_os_error(libc::EBADF));
        }

        if revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP) != 0 {
            return Ok(true);
        }
    }
}

fn thread_function(
    mut rx_transport: XenStoreTransport,
    tx_eventfd: EventFd,
    reply_condvar: Arc<(
        Mutex<VecDeque<Result<XenStoreMessage, std::io::Error>>>,
        Condvar,
    )>,
    watch_condvar: Arc<(
        Mutex<VecDeque<Result<XenStoreMessage, std::io::Error>>>,
        Condvar,
    )>,
) -> Result<(), std::io::Error> {
    loop {
        let mut xen_socket_reply_msg = XenSocketMessage::default();
        let mut buffer: Vec<u8> = vec![0];
        let mut condvar = reply_condvar.clone();
        let mut eventfd: Option<EventFd> = None;

        {
            // SAFETY: `xen_socket_reply_msg` is `XenSocketMessage` bytes sized.
            let xen_socket_reply_msg_slice: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(
                    std::ptr::addr_of_mut!(xen_socket_reply_msg).cast(),
                    mem::size_of::<XenSocketMessage>(),
                )
            };

            rx_transport.read_exact(xen_socket_reply_msg_slice)?;
        }

        if xen_socket_reply_msg.r#type == XS_READ && xen_socket_reply_msg.len == 0 {
            queue_message(
                &condvar,
                eventfd,
                Ok(XenStoreMessage {
                    r#type: xen_socket_reply_msg.r#type,
                    body: "".to_string(),
                }),
            );
            continue;
        }

        buffer.resize(xen_socket_reply_msg.len as usize, 0);

        rx_transport.read_exact(buffer.as_mut_slice())?;

        if xen_socket_reply_msg.r#type != XS_READ
            && xen_socket_reply_msg.r#type != XS_WRITE
            && xen_socket_reply_msg.r#type != XS_WATCH
            && xen_socket_reply_msg.r#type != XS_WATCH_EVENT
            && xen_socket_reply_msg.r#type != XS_DIRECTORY
        {
            queue_message(
                &condvar,
                eventfd,
                Err(Error::other("Xen Store transaction error")),
            );
            continue;
        }

        if xen_socket_reply_msg.r#type == XS_WATCH_EVENT {
            condvar = watch_condvar.clone();
            eventfd = Some(tx_eventfd.try_clone()?);
        }

        match String::from_utf8(buffer) {
            Ok(result) => {
                if result.len() != xen_socket_reply_msg.len as usize {
                    queue_message(&condvar, eventfd, Err(Error::from(ErrorKind::InvalidData)));
                    continue;
                }

                queue_message(
                    &condvar,
                    eventfd,
                    Ok(XenStoreMessage {
                        r#type: xen_socket_reply_msg.r#type,
                        body: result,
                    }),
                );
            }
            Err(e) => {
                queue_message(&condvar, eventfd, Err(Error::other(e)));
            }
        };
    }
}

pub struct XenStoreHandle {
    handler: Option<JoinHandle<Result<(), std::io::Error>>>,
    reply_condvar: Arc<(
        Mutex<VecDeque<Result<XenStoreMessage, std::io::Error>>>,
        Condvar,
    )>,
    watch_condvar: Arc<(
        Mutex<VecDeque<Result<XenStoreMessage, std::io::Error>>>,
        Condvar,
    )>,
    tx_transport: Mutex<XenStoreTransport>,
    rx_eventfd: EventFd,
}

impl XenStoreHandle {
    /// Connect to XenStore through a Unix domain socket or `/dev/xen/xenbus`.
    pub fn new() -> Result<Self, std::io::Error> {
        let transport = default_transport(Path::new(XENSTORED_SOCKET), Path::new(XENBUS_DEVICE))?;
        Self::from_transport(transport)
    }

    fn from_transport(tx_transport: XenStoreTransport) -> Result<Self, std::io::Error> {
        let rx_transport = tx_transport.try_clone()?;
        let tx_eventfd = EventFd::new(EFD_SEMAPHORE)?;
        let rx_eventfd = tx_eventfd.try_clone()?;
        let reply_condvar = Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
        let reply_condvar_cloned = Arc::clone(&reply_condvar);
        let watch_condvar = Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
        let watch_condvar_cloned = Arc::clone(&watch_condvar);

        let handler = thread::spawn(|| {
            thread_function(
                rx_transport,
                tx_eventfd,
                reply_condvar_cloned,
                watch_condvar_cloned,
            )
        });

        Ok(XenStoreHandle {
            handler: Some(handler),
            reply_condvar,
            watch_condvar,
            tx_transport: Mutex::new(tx_transport),
            rx_eventfd,
        })
    }

    fn xs_transaction(&self, r#type: u32, payload: &[u8]) -> Result<String, std::io::Error> {
        let xen_socket_msg = XenSocketMessage::new(r#type, payload.len())?;
        let (lock, cvar) = &*self.reply_condvar;

        let mut tx_transport = self.tx_transport.lock().unwrap();

        // Serialize complete request/response transactions.
        write_request(&mut *tx_transport, &xen_socket_msg, payload)?;

        let mut reply_vec = lock.lock().unwrap();
        while reply_vec.is_empty() {
            reply_vec = cvar.wait(reply_vec).unwrap();
        }

        match reply_vec.pop_front() {
            Some(result) => match result {
                Ok(xsm) => {
                    if xsm.r#type != r#type {
                        return Err(Error::from(ErrorKind::InvalidData));
                    }
                    Ok(xsm.body)
                }
                Err(e) => Err(e),
            },
            None => Err(Error::other("Xen Store transaction error")),
        }
    }

    pub fn read_str(&self, path: &str) -> Result<String, std::io::Error> {
        let payload = CString::new(path)?.into_bytes_with_nul();

        self.xs_transaction(XS_READ, &payload)
    }

    pub fn write_str(&self, path: &str, val: &str) -> Result<(), std::io::Error> {
        let mut payload = CString::new(path)?.into_bytes_with_nul();
        payload.extend(CString::new(val)?.into_bytes());

        self.xs_transaction(XS_WRITE, &payload).map(|_| ())
    }

    pub fn create_watch(&self, path: &str, token: &str) -> Result<(), std::io::Error> {
        let mut payload = CString::new(path)?.into_bytes_with_nul();
        payload.extend(CString::new(token)?.into_bytes_with_nul());

        self.xs_transaction(XS_WATCH, &payload).map(|_| ())
    }

    pub fn read_watch(&self, index: xs_watch_type) -> Result<String, std::io::Error> {
        let (lock, cvar) = &*self.watch_condvar;

        let mut watch_vec = lock.lock().unwrap();
        while watch_vec.is_empty() {
            watch_vec = cvar.wait(watch_vec).unwrap();
        }

        /* Consume eventfd counter incremented in queue_message() */
        let _ = self.rx_eventfd.read().unwrap();

        match watch_vec.pop_front() {
            Some(result) => match result {
                Ok(mut xsm) => {
                    if xsm.r#type != XS_WATCH_EVENT {
                        return Err(Error::from(ErrorKind::InvalidData));
                    }

                    let body = xsm.body.as_mut_str();
                    let v: Vec<&str> = body.split('\0').collect();
                    if index as usize >= v.len() {
                        return Err(Error::from(ErrorKind::InvalidInput));
                    }

                    Ok(String::from(v[index as usize]))
                }
                Err(e) => Err(e),
            },
            None => Err(Error::other("Xen Store transaction error")),
        }
    }

    pub fn fileno(&self) -> Result<i32, std::io::Error> {
        Ok(self.rx_eventfd.as_raw_fd())
    }

    pub fn directory(&self, path: &str) -> Result<Vec<i32>, std::io::Error> {
        let payload = CString::new(path)?.into_bytes_with_nul();

        match self.xs_transaction(XS_DIRECTORY, &payload) {
            Ok(res) => Ok(res
                .as_str()
                .split('\0')
                .filter(|v| !v.is_empty())
                .map(|v| {
                    v.parse::<i32>()
                        .map_err(|err| format!("Could not parse `{:?}` as `i32`: {err}", v))
                        .unwrap()
                })
                .collect()),
            Err(e) => Err(e),
        }
    }
}

impl Drop for XenStoreHandle {
    fn drop(&mut self) {
        let tx_transport = self.tx_transport.lock().unwrap();

        tx_transport.shutdown();

        /* Wait for it to stop */
        let _ = self.handler.take().unwrap().join();
    }
}

#[cfg(test)]
mod tests {
    use std::{os::fd::OwnedFd, sync::mpsc, time::Duration};

    use vmm_sys_util::tempdir::TempDir;

    use super::*;

    fn device_transport_from_stream(
        stream: UnixStream,
    ) -> Result<XenStoreTransport, std::io::Error> {
        stream.set_nonblocking(true)?;

        Ok(XenStoreTransport::Device {
            file: File::from(OwnedFd::from(stream)),
            stop_eventfd: EventFd::new(0)?,
        })
    }

    #[derive(Default)]
    struct ShortWriter {
        bytes: Vec<u8>,
        write_calls: usize,
    }

    impl Write for ShortWriter {
        fn write(&mut self, buffer: &[u8]) -> Result<usize, std::io::Error> {
            self.write_calls += 1;

            // Write the header, interrupt the payload, then force short writes.
            let written = match self.write_calls {
                1 => buffer.len(),
                2 => return Err(Error::from(ErrorKind::Interrupted)),
                _ => buffer.len().min(3),
            };

            self.bytes.extend_from_slice(&buffer[..written]);
            Ok(written)
        }

        fn flush(&mut self) -> Result<(), std::io::Error> {
            Ok(())
        }
    }

    #[test]
    fn writes_complete_request_after_interrupted_and_short_writes() -> Result<(), std::io::Error> {
        let payload = b"domid\0";
        let message = XenSocketMessage::new(XS_READ, payload.len())?;
        let mut writer = ShortWriter::default();

        write_request(&mut writer, &message, payload)?;

        let mut expected = message_bytes(&message).to_vec();
        expected.extend_from_slice(payload);
        assert_eq!(writer.bytes, expected);
        assert!(writer.write_calls > 2);
        Ok(())
    }

    #[test]
    fn unix_transport_round_trip() -> Result<(), std::io::Error> {
        let (client, mut server) = UnixStream::pair()?;
        let handle = XenStoreHandle::from_transport(XenStoreTransport::Socket(client))?;
        // Keep the peer open until the handle shuts down its reader.
        let (release_sender, release_receiver) = mpsc::channel();

        let server_thread = thread::spawn(move || -> Result<(), std::io::Error> {
            let mut request = XenSocketMessage::default();
            // SAFETY: request provides exclusive storage for one XenSocketMessage.
            let request_bytes = unsafe {
                std::slice::from_raw_parts_mut(
                    std::ptr::addr_of_mut!(request).cast(),
                    mem::size_of::<XenSocketMessage>(),
                )
            };
            server.read_exact(request_bytes)?;
            assert_eq!(request.r#type, XS_READ);

            let mut payload = vec![0; request.len as usize];
            server.read_exact(&mut payload)?;
            assert_eq!(payload, b"domid\0");

            let response_payload = b"7\0";
            let response = XenSocketMessage::new(XS_READ, response_payload.len())?;
            server.write_all(message_bytes(&response))?;
            server.write_all(response_payload)?;
            release_receiver.recv().unwrap();
            Ok(())
        });

        assert_eq!(handle.read_str("domid")?, "7\0");
        drop(handle);
        release_sender.send(()).unwrap();
        server_thread.join().unwrap()?;
        Ok(())
    }

    #[test]
    fn dropping_idle_device_handle_stops_reader() -> Result<(), std::io::Error> {
        // Keep the peer open so shutdown alone stops the reader.
        let (client, _server) = UnixStream::pair()?;
        let handle = XenStoreHandle::from_transport(device_transport_from_stream(client)?)?;
        drop(handle);
        Ok(())
    }

    #[test]
    fn device_read_stops_after_partial_data() -> Result<(), std::io::Error> {
        let (client, mut server) = UnixStream::pair()?;
        let transport = device_transport_from_stream(client)?;
        let mut rx_transport = transport.try_clone()?;
        let (read_sender, read_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();

        let reader = thread::spawn(move || {
            let mut buffer = [0; 4];
            let read = rx_transport.read(&mut buffer).unwrap();
            read_sender.send(read).unwrap();
            let result = rx_transport.read_exact(&mut buffer[read..]);
            result_sender.send((result, buffer)).unwrap();
        });

        server.write_all(b"ab")?;
        assert_eq!(
            read_receiver
                .recv_timeout(Duration::from_secs(1))
                .map_err(|error| Error::new(ErrorKind::TimedOut, error))?,
            2
        );
        transport.shutdown();

        let (result, buffer) = result_receiver
            .recv_timeout(Duration::from_secs(1))
            .map_err(|error| Error::new(ErrorKind::TimedOut, error))?;
        assert_eq!(result.unwrap_err().kind(), ErrorKind::UnexpectedEof);
        assert_eq!(&buffer[..2], b"ab");
        reader.join().unwrap();
        Ok(())
    }

    #[test]
    fn device_read_honors_shutdown_with_ready_input() -> Result<(), std::io::Error> {
        let (client, mut server) = UnixStream::pair()?;
        let transport = device_transport_from_stream(client)?;
        let mut rx_transport = transport.try_clone()?;

        server.write_all(b"abcd")?;
        transport.shutdown();

        let mut buffer = [0; 4];
        assert_eq!(
            rx_transport.read_exact(&mut buffer).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
        Ok(())
    }

    #[test]
    fn default_transport_falls_back_to_device() -> Result<(), std::io::Error> {
        let temp_dir = TempDir::new()?;
        let socket_path = temp_dir.as_path().join("missing-socket");
        let device_path = temp_dir.as_path().join("xenbus");
        let _device = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&device_path)?;

        let transport = default_transport(&socket_path, &device_path)?;
        assert!(matches!(transport, XenStoreTransport::Device { .. }));
        Ok(())
    }
}
