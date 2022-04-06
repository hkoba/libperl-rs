#![allow(non_snake_case)]

#[cfg(perlapi_ver26)]
use std::convert::TryFrom;

pub use libperl_sys::op;

use libperl_sys::*;
use libperl_rs::Perl;

use super::sv0::{Sv, sv_extract};
use super::pad0::*;

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum Op/* <'a>*/ {
    NULL,
    OP (opcode, Option<String>, Option<String>),
    UNOP (opcode/*, &'a unop*/),
    BINOP (opcode/*, &'a binop*/),
    LOGOP (opcode/*, &'a logop*/),
    LISTOP (opcode/*, &'a listop*/),
    PMOP (opcode/*, &'a pmop*/),
    SVOP (opcode, Sv),
    PADOP (opcode, Sv),
    PVOP (opcode/*, &'a pvop*/),
    LOOP (opcode/*, &'a loop_*/),
    COP (opcode/*, &'a cop*/),
    METHOP (opcode, Name),
    #[cfg(perlapi_ver26)]
    UNOP_AUX(opcode),
}

#[derive(Debug)]
pub enum Name {
    Dynamic,
    Const(Sv),
}

#[cfg(perlapi_ver26)]
pub fn op_extract(perl: &Perl, cv: *const cv, o: *const op) -> Op {
    let cls = perl.op_class(o);
    let oc = opcode::try_from(o).unwrap();
    match cls {
        OPclass::OPclass_NULL => Op::NULL,
        OPclass::OPclass_BASEOP => {
            let op = unsafe {o.as_ref().unwrap()};
            if let Some(pl) = cv_padnamelist(cv) {
                if let Some(padname) = padnamelist_nth(pl, op.op_targ as usize) {
                    return Op::OP(oc, PadnamePV(padname), PadnameTYPE(padname))
                }
            }
            Op::OP(oc, None, None)
        },
        OPclass::OPclass_UNOP => Op::UNOP(oc/*, unsafe {(o as *const unop).as_ref()}.unwrap()*/),
        OPclass::OPclass_BINOP => Op::BINOP(oc/*, unsafe {(o as *const binop).as_ref()}.unwrap()*/),
        OPclass::OPclass_LOGOP => Op::LOGOP(oc/*, unsafe {(o as *const logop).as_ref()}.unwrap()*/),
        OPclass::OPclass_LISTOP => Op::LISTOP(oc/*, unsafe {(o as *const listop).as_ref()}.unwrap()*/),
        OPclass::OPclass_PMOP => Op::PMOP(oc/*, unsafe {(o as *const pmop).as_ref()}.unwrap()*/),
        OPclass::OPclass_SVOP => {
            let sv = op_sv_or(o, |op| PAD_BASE_SV(CvPADLIST(cv), op.op_targ));
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
        OPclass::OPclass_METHOP => {
            if (unsafe {*o}.op_flags & OPf_KIDS as u8) != 0 {
                Op::METHOP(oc, Name::Dynamic)
                
            } else {
                let sv = op_sv_or(o, |op| PAD_BASE_SV(CvPADLIST(cv), op.op_targ));
                Op::METHOP(oc, Name::Const(sv_extract(sv)))
            }
        },
        #[cfg(perlapi_ver26)]
        OPclass::OPclass_UNOP_AUX => Op::UNOP_AUX(oc /*, unsafe {(o as *const unop_aux).as_ref()}.unwrap()*/),
    }
}
    
pub fn op_sv_or<F>(op: *const op, f: F) -> *const libperl_sys::sv
    where F: Fn(&svop) -> *const libperl_sys::sv
{
    let svop = unsafe {(op as *const svop).as_ref()}.unwrap();
    if !svop.op_sv.is_null() {
        svop.op_sv
    } else {
        f(svop)
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
        if u32::try_from(op.op_moresib()).unwrap() == 1 as u32 {
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
