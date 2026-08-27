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
    io::{Error, ErrorKind, Read, Write},
    mem,
    net::Shutdown,
    os::unix::{io::AsRawFd, net::UnixStream},
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

fn thread_function(
    mut rx_socket: UnixStream,
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

            rx_socket.read_exact(xen_socket_reply_msg_slice)?;
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

        rx_socket.read_exact(buffer.as_mut_slice())?;

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
    tx_socket: Mutex<UnixStream>,
    rx_eventfd: EventFd,
}

impl XenStoreHandle {
    pub fn new() -> Result<Self, std::io::Error> {
        let tx_socket = UnixStream::connect(XENSTORED_SOCKET)?;
        let rx_socket = tx_socket.try_clone()?;
        let tx_eventfd = EventFd::new(EFD_SEMAPHORE)?;
        let rx_eventfd = tx_eventfd.try_clone()?;
        let reply_condvar = Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
        let reply_condvar_cloned = Arc::clone(&reply_condvar);
        let watch_condvar = Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
        let watch_condvar_cloned = Arc::clone(&watch_condvar);

        let handler = thread::spawn(|| {
            thread_function(
                rx_socket,
                tx_eventfd,
                reply_condvar_cloned,
                watch_condvar_cloned,
            )
        });

        Ok(XenStoreHandle {
            handler: Some(handler),
            reply_condvar,
            watch_condvar,
            tx_socket: Mutex::new(tx_socket),
            rx_eventfd,
        })
    }

    fn xs_transaction(&self, r#type: u32, payload: &[u8]) -> Result<String, std::io::Error> {
        let xen_socket_msg = XenSocketMessage::new(r#type, payload.len())?;
        let (lock, cvar) = &*self.reply_condvar;

        let mut tx_socket = self.tx_socket.lock().unwrap();

        // Serialize complete request/response transactions.
        write_request(&mut *tx_socket, &xen_socket_msg, payload)?;

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
        let tx_socket = self.tx_socket.lock().unwrap();

        /*
         * Calling shutdown() on the socket will cause the blocking
         * rx_socket in thread_function() to return with an error
         * condition, something that will automatically break the
         * loop and cause the thread to stop.
         */
        let _ = tx_socket.shutdown(Shutdown::Both);

        /* Wait for it to stop */
        let _ = self.handler.take().unwrap().join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
