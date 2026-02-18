use xen_sys::{
    bindings::{
        EVTCHNOP_alloc_unbound, EVTCHNOP_bind_interdomain, EVTCHNOP_close, EVTCHNOP_send,
        __HYPERVISOR_event_channel_op,
    },
    error::XenError,
    DomId, XenConstBuffer, XenHypercall, XenMutBuffer,
};

use crate::event::{EventChannel, EventChannelInterface};

pub struct RawEventChannelInterface<H: XenHypercall>(H);

#[derive(Clone, Copy)]
#[repr(C)]
struct EvtchnAllocUnbound {
    dom: DomId,
    remote_dom: DomId,
    port: EventChannel,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct EvtchnBindInterdomain {
    remote_dom: DomId,
    remote_port: EventChannel,
    local_port: EventChannel,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct EvtchnParam {
    port: EventChannel,
}

impl<H: XenHypercall> RawEventChannelInterface<H> {
    pub fn new(hyp: H) -> Self {
        Self(hyp)
    }
}

impl<H: XenHypercall> EventChannelInterface for RawEventChannelInterface<H> {
    type Error = XenError;

    /// Allocate a port in this domain and mark as accepting interdomain
    /// bindings from domain `remote_dom`.
    fn alloc(&mut self, remote_dom: DomId) -> Result<EventChannel, Self::Error> {
        let mut alloc_unbound = EvtchnAllocUnbound {
            dom: DomId::SELF,
            remote_dom,
            port: EventChannel(0), // OUT
        };

        unsafe {
            let mut alloc_unbound_buffer = self.0.make_mut_object(&mut alloc_unbound)?;
            self.0.hypercall2(
                __HYPERVISOR_event_channel_op as _,
                [
                    EVTCHNOP_alloc_unbound as _,
                    alloc_unbound_buffer.as_hypercall_ptr().addr(),
                ],
            )?;
            alloc_unbound_buffer.update();
        }

        Ok(alloc_unbound.port)
    }

    /// Construct an interdomain event channel between
    /// the calling domain and `remote_dom`. `remote_dom`,`remote_port` must
    /// identify a port that is unbound and marked as accepting bindings
    /// from the calling domain. A fresh port is allocated in the calling
    /// domain and returned.
    fn bind_interdomain(
        &mut self,
        remote_dom: DomId,
        remote_port: EventChannel,
    ) -> Result<EventChannel, Self::Error> {
        let mut bind_interdomain = EvtchnBindInterdomain {
            remote_dom,
            remote_port,
            local_port: EventChannel(0), // OUT
        };

        unsafe {
            let mut bind_interdomain_buffer = self.0.make_mut_object(&mut bind_interdomain)?;
            self.0.hypercall2(
                __HYPERVISOR_event_channel_op as _,
                [
                    EVTCHNOP_bind_interdomain as _,
                    bind_interdomain_buffer.as_hypercall_ptr().addr(),
                ],
            )?;
            bind_interdomain_buffer.update();
        }

        Ok(bind_interdomain.local_port)
    }

    fn unbind(&mut self, port: EventChannel) -> Result<(), Self::Error> {
        let close = EvtchnParam { port };
        unsafe {
            let close_buffer = self.0.make_const_object(&close)?;
            self.0.hypercall2(
                __HYPERVISOR_event_channel_op as _,
                [EVTCHNOP_close as _, close_buffer.as_hypercall_ptr().addr()],
            )?;
        }

        Ok(())
    }

    /// Send a event to a event channel port.
    fn send(&self, port: EventChannel) -> Result<(), Self::Error> {
        let send = EvtchnParam { port };
        unsafe {
            let send_buffer = self.0.make_const_object(&send)?;
            self.0.hypercall2(
                __HYPERVISOR_event_channel_op as _,
                [EVTCHNOP_send as _, send_buffer.as_hypercall_ptr().addr()],
            )?;
        }

        Ok(())
    }
}
