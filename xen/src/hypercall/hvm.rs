use xen_sys::{
    bindings::{HVMOP_get_param, __HYPERVISOR_hvm_op, xen_hvm_param},
    error::XenError,
    DomId, XenHypercall, XenMutBuffer,
};

pub trait XenHvmOp: XenHypercall {
    fn get_hvm_param(&self, index: u32) -> Result<u64, XenError> {
        let mut param = xen_hvm_param {
            domid: DomId::SELF.0,
            index,
            pad: 0,
            value: 0,
        };
        let mut param_buffer = self.make_mut_object(&mut param)?;

        unsafe {
            self.hypercall2(
                __HYPERVISOR_hvm_op as _,
                [HVMOP_get_param as _, param_buffer.as_hypercall_ptr().addr()],
            )?;
            param_buffer.update();
            drop(param_buffer);
        };

        Ok(param.value)
    }
}

impl<H: XenHypercall> XenHvmOp for H {}
