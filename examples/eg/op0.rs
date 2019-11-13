use libperl_sys::*;
use libperl_rs::Perl;

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

#[cfg(perlapi_ver26)]
pub fn op_extract(perl: &Perl, o: *const op) -> Op {
    let cls = perl.op_class(o);
    let oc = unsafe {
        let ty = (*o).op_type();
        *(&ty as *const u32 as *const opcode)
    };
    match cls {
        OPclass::OPclass_NULL => Op::NULL,
        OPclass::OPclass_BASEOP => Op::OP(oc, unsafe {o.as_ref()}.unwrap()),
        OPclass::OPclass_UNOP => Op::UNOP(oc, unsafe {(o as *const unop).as_ref()}.unwrap()),
        OPclass::OPclass_BINOP => Op::BINOP(oc, unsafe {(o as *const binop).as_ref()}.unwrap()),
        OPclass::OPclass_LOGOP => Op::LOGOP(oc, unsafe {(o as *const logop).as_ref()}.unwrap()),
        OPclass::OPclass_LISTOP => Op::LISTOP(oc, unsafe {(o as *const listop).as_ref()}.unwrap()),
        OPclass::OPclass_PMOP => Op::PMOP(oc, unsafe {(o as *const pmop).as_ref()}.unwrap()),
        OPclass::OPclass_SVOP => Op::SVOP(oc, unsafe {(o as *const svop).as_ref()}.unwrap()),
        OPclass::OPclass_PADOP => Op::PADOP(oc, unsafe {(o as *const padop).as_ref()}.unwrap()),
        OPclass::OPclass_PVOP => Op::PVOP(oc, unsafe {(o as *const pvop).as_ref()}.unwrap()),
        OPclass::OPclass_LOOP => Op::LOOP(oc, unsafe {(o as *const loop_).as_ref()}.unwrap()),
        OPclass::OPclass_COP => Op::COP(oc, unsafe {(o as *const cop).as_ref()}.unwrap()),
        OPclass::OPclass_METHOP => Op::METHOP(oc, unsafe {(o as *const methop).as_ref()}.unwrap()),
        //        OPclass::OPclass_UNOP_AUX => Op::UNOP_AUX(oc, unsafe {(o as *const unop_aux).as_ref()}.unwrap()),
        _ => panic!("Unknown op type {:#?}", o),
    }
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
