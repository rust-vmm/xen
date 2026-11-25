/*
 * Copyright 2022-23 Mathieu Poirier <mathieu.poirier@linaro.org>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use std::io;

pub const XENSTORED_SOCKET: &str = "/var/run/xenstored/socket";
pub const XENBUS_DEVICE: &str = "/dev/xen/xenbus";
pub const XENSTORE_PAYLOAD_MAX: u32 = 4096;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
// xen/include/public/io/xs_wire.h::struct xsd_sockmsg
pub(crate) struct XenSocketMessage {
    pub r#type: u32,
    pub req_id: u32,
    pub tx_id: u32,
    pub len: u32,
}

impl XenSocketMessage {
    pub(crate) fn new(r#type: u32, payload_len: usize) -> Result<Self, std::io::Error> {
        if payload_len > XENSTORE_PAYLOAD_MAX as usize {
            return Err(io::Error::from_raw_os_error(libc::E2BIG));
        }

        let msg = XenSocketMessage {
            r#type,
            req_id: 0,
            tx_id: 0,
            len: payload_len as u32,
        };

        Ok(msg)
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct XenStoreMessage {
    pub r#type: u32,
    pub body: String,
}
