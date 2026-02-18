// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 Vates SAS - Teddy Astie

use core::{fmt, ptr::NonNull};

use xen::{
    event::{EventChannel, EventChannelInterface, RawEventChannelInterface},
    hypercall::hvm::XenHvmOp,
    io::{XenConsInterface, XenConsInterfaceVolatileFieldAccess, ring::XenRing},
};
use xen_sys::{
    NativeXenHypercall,
    bindings::{HVM_PARAM_CONSOLE_EVTCHN, HVM_PARAM_CONSOLE_PFN},
};

use crate::{arch::map_4k_frame, delay};

pub struct XenConsole {
    pub interface: XenRing<'static>,
    evtchn_device: RawEventChannelInterface<NativeXenHypercall>,
    event_channel: EventChannel,
}

unsafe impl Send for XenConsole {}
unsafe impl Sync for XenConsole {}

impl XenConsole {
    pub unsafe fn new() -> Option<Self> {
        let hyp = unsafe { NativeXenHypercall::new()? };

        let pfn = hyp.get_hvm_param(HVM_PARAM_CONSOLE_PFN).ok()?;
        let evtchn = hyp.get_hvm_param(HVM_PARAM_CONSOLE_EVTCHN).ok()?;

        if pfn == 0 {
            return None;
        }

        let console = unsafe { XenConsInterface::new(map_4k_frame(pfn, false)?) };

        console.out_prod().write(1);

        Some(Self {
            interface: XenConsInterface::to_ring(console),
            evtchn_device: RawEventChannelInterface::new(hyp),
            event_channel: EventChannel(evtchn as u32),
        })
    }

    /// Move the PV console to another address/event channel.
    pub unsafe fn relocate(
        &mut self,
        addr: Option<NonNull<XenConsInterface>>,
        event_channel: Option<EventChannel>,
    ) {
        if let Some(addr) = addr {
            self.interface = XenConsInterface::to_ring(unsafe { XenConsInterface::new(addr) });
        }

        if let Some(event_channel) = event_channel {
            self.event_channel = event_channel;
        }
    }
}

impl fmt::Write for XenConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut write_buffer = |mut line: &[u8]| {
            while !line.is_empty() {
                delay::wait_until(50, || {
                    let available = self.interface.available();
                    if available > 0 {
                        true
                    } else {
                        self.evtchn_device.send(self.event_channel).ok();
                        false
                    }
                });

                let available = self.interface.available();
                let (part, after) = line.split_at(available.min(line.len()));

                self.interface.write(part).ok();
                line = after;
            }
        };

        for mut line in s.split_inclusive('\n').map(|s| s.as_bytes()) {
            let newline = line.last() == Some(&b'\n');
            if newline {
                line = &line[..line.len() - 1];
            }

            write_buffer(line);

            if newline {
                write_buffer(&[b'\r', b'\n']);
            }
        }

        Ok(())
    }
}
