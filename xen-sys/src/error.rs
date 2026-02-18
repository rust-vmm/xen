#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum XenError {
    #[error("Xen error")]
    Xen(isize),
    #[error("Other error")]
    Other(&'static str),
}

pub fn parse_hypercall_return(ret: isize) -> Result<usize, XenError> {
    if ret < 0 {
        Err(XenError::Xen(ret))
    } else {
        Ok(ret as usize)
    }
}
