mod ioctl;

extern crate std;

use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd},
        unix::prelude::RawFd,
    },
};

use xen::event::{EventChannel, EventChannelInterface, SyncEventChannelReceiver};
use xen_sys::DomId;

const EVTCHN_PATH: &str = "/dev/xen/evtchn";

pub struct UnixEventChannelInterface(File);

impl UnixEventChannelInterface {
    pub fn new() -> io::Result<Self> {
        Ok(Self(
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(EVTCHN_PATH)?,
        ))
    }
}

impl EventChannelInterface for UnixEventChannelInterface {
    type Error = io::Error;

    fn alloc(&mut self, remote_dom: DomId) -> Result<EventChannel, Self::Error> {
        let arg = ioctl::BindUnboundPortArg {
            remote_domain: remote_dom.0 as _,
        };

        // SAFETY: `data` is properly sized for this ioctl.
        let port = unsafe { ioctl::event_bind_unbound_port(self.0.as_raw_fd(), &arg) }?;

        assert!(port >= 0);

        Ok(EventChannel(port as u32))
    }

    fn bind_interdomain(
        &mut self,
        remote_dom: DomId,
        remote_port: EventChannel,
    ) -> Result<EventChannel, Self::Error> {
        let arg = ioctl::BindInterdomainArg {
            remote_domain: remote_dom.0 as _,
            remote_port: remote_port.0 as _,
        };

        // SAFETY: `data` is properly sized for this ioctl.
        let port = unsafe { ioctl::event_bind_interdomain(self.0.as_raw_fd(), &arg) }?;

        assert!(port >= 0);

        Ok(EventChannel(port as u32))
    }

    fn unbind(&mut self, port: EventChannel) -> Result<(), Self::Error> {
        let arg = ioctl::UnbindPortArg { port: port.0 as _ };

        // SAFETY: `arg` is properly sized for this ioctl.
        unsafe { ioctl::event_unbind_port(self.0.as_raw_fd(), &arg) }.ok();

        Ok(())
    }

    fn send(&self, port: EventChannel) -> Result<(), Self::Error> {
        let arg = ioctl::NotifyArg { port: port.0 as _ };

        // SAFETY: `data` is properly sized for this ioctl.
        unsafe { ioctl::notify_port(self.0.as_raw_fd(), &arg) }?;

        Ok(())
    }
}

impl SyncEventChannelReceiver for UnixEventChannelInterface {
    fn pending(&mut self) -> Result<EventChannel, Self::Error> {
        let mut bytes = [0; 4];
        self.0.read_exact(&mut bytes)?;

        Ok(EventChannel(<u32>::from_ne_bytes(bytes)))
    }

    fn unmask(&mut self, evtchn: EventChannel) -> Result<(), Self::Error> {
        self.0.write_all(&evtchn.0.to_ne_bytes())
    }
}

impl AsFd for UnixEventChannelInterface {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for UnixEventChannelInterface {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}
