use nix::{ioc, ioctl_write_ptr_bad, libc::c_uint};

#[repr(C)]
pub struct BindInterdomainArg {
    pub remote_domain: c_uint,
    pub remote_port: c_uint,
}
ioctl_write_ptr_bad!(
    event_bind_interdomain,
    ioc!(0, b'E', 1, size_of::<BindInterdomainArg>()),
    BindInterdomainArg
);

#[repr(C)]
pub struct BindUnboundPortArg {
    pub remote_domain: c_uint,
}

ioctl_write_ptr_bad!(
    event_bind_unbound_port,
    ioc!(0, b'E', 2, size_of::<BindUnboundPortArg>()),
    BindUnboundPortArg
);

#[repr(C)]
pub struct UnbindPortArg {
    pub port: c_uint,
}
ioctl_write_ptr_bad!(
    event_unbind_port,
    ioc!(0, b'E', 3, size_of::<UnbindPortArg>()),
    UnbindPortArg
);

#[repr(C)]
pub struct NotifyArg {
    pub port: c_uint,
}
ioctl_write_ptr_bad!(
    notify_port,
    ioc!(0, b'E', 4, size_of::<NotifyArg>()),
    NotifyArg
);
