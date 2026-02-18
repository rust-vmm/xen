use core::{error::Error, future::Future};

use xen_sys::DomId;

mod none;
pub use none::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EventChannel(pub u32);

pub trait EventChannelInterface {
    type Error: Error;

    /// Allocate a port in this domain and mark as accepting interdomain
    /// bindings from domain `remote_dom`.
    fn alloc(&mut self, remote_dom: DomId) -> Result<EventChannel, Self::Error>;

    /// Construct an interdomain event channel between
    /// the calling domain and `remote_dom`. `remote_dom`,`remote_port` must
    /// identify a port that is unbound and marked as accepting bindings
    /// from the calling domain. A fresh port is allocated in the calling
    /// domain and returned.
    fn bind_interdomain(
        &mut self,
        remote_dom: DomId,
        remote_port: EventChannel,
    ) -> Result<EventChannel, Self::Error>;

    fn unbind(&mut self, evtchn: EventChannel) -> Result<(), Self::Error>;

    /// Send a event to a event channel port.
    fn send(&self, port: EventChannel) -> Result<(), Self::Error>;
}

pub trait SyncEventChannelReceiver: EventChannelInterface {
    /// Read the pending event.
    fn pending(&mut self) -> Result<EventChannel, Self::Error>;

    /// Unmask the event channel, making it ready for receiving another event.
    fn unmask(&mut self, port: EventChannel) -> Result<(), Self::Error>;
}

pub trait AsyncEventChannelReceiver: EventChannelInterface {
    /// Read the pending event.
    fn pending(&mut self) -> impl Future<Output = Result<EventChannel, Self::Error>> + Send;

    /// Unmask the event channel, making it ready for receiving another event.
    fn unmask(
        &mut self,
        port: EventChannel,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
