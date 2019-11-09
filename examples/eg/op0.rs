use libperl_sys::*;

#[cfg(perlapi_ver26)]
pub fn op_sibling(op: *const unop) -> *const op {
    // PERL_OP_PARENT is on since 5.26
    if let Some(op) = unsafe {op.as_ref()} {
        if op.op_moresib() == 1 as u32 {
            op.op_sibparent
        } else {
            std::ptr::null()
        }
    } else {
        std::ptr::null()
    }
}

#[cfg(not(perlapi_ver26))]
pub fn op_sibling(op: *const unop) -> *const op {
    if let Some(op) = unsafe {op.as_ref()} {
        op.op_sibling
    } else {
        std::ptr::null()
    }
}
