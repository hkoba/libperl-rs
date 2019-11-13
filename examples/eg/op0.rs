use libperl_sys::*;

#[derive(Debug)]
pub enum Op<'a> {
    NULL,
    OP (opcode, &'a op),
    UNOP (opcode, &'a unop),
    BINOP (opcode, &'a binop),
    LOGOP (opcode, &'a logop),
    LISTOP (opcode, &'a listop),
    PMOP (opcode, &'a pmop),
    SVOP (opcode, &'a svop),
    PADOP (opcode, &'a padop),
    PVOP (opcode, &'a pvop),
    LOOP (opcode, &'a loop_),
    COP (opcode, &'a cop),
    METHOP (opcode, &'a methop),
}

pub fn op_name(o: *const op) -> String {
    let ty = unsafe {*o}.op_type();
    unsafe {
        std::ffi::CStr::from_ptr(PL_op_name[ty as usize])
    }.to_str().unwrap().to_string()
}

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

pub fn op_first(o: *const op) -> *const op {
    if o.is_null() || (unsafe {*o}.op_flags & OPf_KIDS as u8) == 0 {
        std::ptr::null()
    } else {
        let uo = o as *const unop;
        unsafe {*uo}.op_first as *const op
    }
}
