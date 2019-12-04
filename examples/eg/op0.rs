pub use libperl_sys::op;

use libperl_sys::*;
use libperl_rs::Perl;

use super::sv0::{Sv, sv_extract};
use super::pad0::*;

#[derive(Debug)]
pub enum Op/* <'a>*/ {
    NULL,
    OP (opcode/*, &'a op*/),
    UNOP (opcode/*, &'a unop*/),
    BINOP (opcode/*, &'a binop*/),
    LOGOP (opcode/*, &'a logop*/),
    LISTOP (opcode/*, &'a listop*/),
    PMOP (opcode/*, &'a pmop*/),
    SVOP (opcode, Sv/*<'a>*/),
    PADOP (opcode, Sv/*<'a>*/),
    PVOP (opcode/*, &'a pvop*/),
    LOOP (opcode/*, &'a loop_*/),
    COP (opcode/*, &'a cop*/),
    METHOP (opcode/*, &'a methop*/),
}

#[cfg(perlapi_ver26)]
pub fn op_extract(perl: &Perl, cv: *const cv, o: *const op) -> Op {
    let cls = perl.op_class(o);
    let oc = unsafe {
        let ty = (*o).op_type();
        *(&ty as *const u32 as *const opcode)
    };
    match cls {
        OPclass::OPclass_NULL => Op::NULL,
        OPclass::OPclass_BASEOP => Op::OP(oc/*, unsafe {o.as_ref()}.unwrap()*/),
        OPclass::OPclass_UNOP => Op::UNOP(oc/*, unsafe {(o as *const unop).as_ref()}.unwrap()*/),
        OPclass::OPclass_BINOP => Op::BINOP(oc/*, unsafe {(o as *const binop).as_ref()}.unwrap()*/),
        OPclass::OPclass_LOGOP => Op::LOGOP(oc/*, unsafe {(o as *const logop).as_ref()}.unwrap()*/),
        OPclass::OPclass_LISTOP => Op::LISTOP(oc/*, unsafe {(o as *const listop).as_ref()}.unwrap()*/),
        OPclass::OPclass_PMOP => Op::PMOP(oc/*, unsafe {(o as *const pmop).as_ref()}.unwrap()*/),
        OPclass::OPclass_SVOP => {
            let op = unsafe {(o as *const svop).as_ref()}.unwrap();
            let sv = if !op.op_sv.is_null() {
                op.op_sv
            } else {
                PAD_BASE_SV(CvPADLIST(cv), op.op_targ)
            };
            Op::SVOP(oc, sv_extract(sv))
        },
        OPclass::OPclass_PADOP => {
            let op = unsafe {(o as *const padop).as_ref()}.unwrap();
            let sv = PAD_BASE_SV(CvPADLIST(cv), op.op_padix);
            Op::PADOP(oc, sv_extract(sv))
        },
        OPclass::OPclass_PVOP => Op::PVOP(oc/*, unsafe {(o as *const pvop).as_ref()}.unwrap()*/),
        OPclass::OPclass_LOOP => Op::LOOP(oc/*, unsafe {(o as *const loop_).as_ref()}.unwrap()*/),
        OPclass::OPclass_COP => Op::COP(oc/*, unsafe {(o as *const cop).as_ref()}.unwrap()*/),
        OPclass::OPclass_METHOP => Op::METHOP(oc/*, unsafe {(o as *const methop).as_ref()}.unwrap()*/),
        //        OPclass::OPclass_UNOP_AUX => Op::UNOP_AUX(oc, unsafe {(o as *const unop_aux).as_ref()}.unwrap()),
        _ => panic!("Unknown op type {:#?}", o),
    }
}
    
pub fn next_iter(op: *const op) -> OpNextIter {
    OpNextIter {op}
}

pub struct OpNextIter {
    op: *const op,
}

impl Iterator for OpNextIter {
    type Item = *const op;
    
    fn next(&mut self) -> Option<Self::Item> {
        let op = self.op;
        if op.is_null() {
            None
        } else {
            self.op = unsafe {(*op).op_next as *const op};
            Some(op)
        }
    }
}

pub fn op_name(o: *const op) -> String {
    let ty = unsafe {*o}.op_type();
    unsafe {
        std::ffi::CStr::from_ptr(PL_op_name[ty as usize])
    }.to_str().unwrap().to_string()
}

pub fn sibling_iter(op: *const op) -> OpSiblingIter {
    OpSiblingIter {op: op_first(op)}
}

pub struct OpSiblingIter {
    op: *const op,
}

impl Iterator for OpSiblingIter {
    type Item = *const op;
    
    fn next(&mut self) -> Option<Self::Item> {
        let op = self.op;
        if op.is_null() {
            None
        } else {
            self.op = op_sibling(op as *const unop);
            Some(op)
        }
    }
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
