#![allow(non_snake_case)]
pub use libperl_sys::op;

use libperl_sys::*;
use libperl_rs::Perl;

use super::sv0::{Sv, sv_extract};
use super::pad0::*;
use super::op0::{op_sibling, op_sv_or, Name};
pub use super::op0::op_name;

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum Op/* <'a>*/ {
    NULL,
    OP {opcode: opcode, sibling: Box<Op>, name: Option<String>, typ: Option<String>},
    UNOP {opcode: opcode, sibling: Box<Op>, first: Box<Op>},
    BINOP {opcode: opcode, sibling: Box<Op>, first: Box<Op>, last: Box<Op>},
    LOGOP {opcode: opcode// , sibling: Box<Op>, first: Box<Op>, other: Box<Op>
    },
    LISTOP {opcode: opcode// , sibling: Box<Op>, first: Box<Op>, last: Box<Op>
    },
    PMOP {opcode: opcode// , sibling: Box<Op>, first: Box<Op>, last: Box<Op>
    },
    SVOP (opcode, Sv),
    PADOP (opcode, Sv),
    PVOP (opcode/*, &'a pvop*/),
    LOOP {opcode: opcode// , sibling: Box<Op>, first: Box<Op>, last: Box<Op>, redoop: Box<Op>, next: Box<Op>, last: Box<Op>
    },
    COP (opcode/*, &'a cop*/),
    METHOP(opcode, Name),
    // {opcode: opcode, sibling: Box<Op>, name: Name},
    #[cfg(perlapi_ver26)]
    UNOP_AUX(opcode),
}

#[cfg(perlapi_ver26)]
pub fn op_extract(perl: &Perl, cv: *const cv, o: *const op) -> Op {
    if o.is_null() {
        return Op::NULL
    }
    let cls = perl.op_class(o);
    let oc = unsafe {
        let ty = (*o).op_type();
        *(&ty as *const u32 as *const opcode)
    };
    match cls {
        OPclass::OPclass_NULL => Op::NULL,
        OPclass::OPclass_BASEOP => {
            let op = unsafe {o.as_ref().unwrap()};
            let sibling = Box::new(op_extract(&perl, cv, op_sibling(o as *const unop)));
            if let Some(pl) = cv_padnamelist(cv) {
                if let Some(padname) = padnamelist_nth(pl, op.op_targ as usize) {
                    return Op::OP {
                        opcode: oc, name: PadnamePV(padname), typ: PadnameTYPE(padname),
                        sibling
                    }
                }
            }
            Op::OP {opcode: oc, name: None, typ: None, sibling}
        },
        OPclass::OPclass_UNOP => {
            let op = unsafe {(o as *const unop).as_ref()}.unwrap();
            let sibling = Box::new(op_extract(&perl, cv, op_sibling(o as *const unop)));
            Op::UNOP {opcode: oc, sibling, first: Box::new(op_extract(&perl, cv, op.op_first))}
        },
        OPclass::OPclass_BINOP => {
            let op = unsafe {(o as *const binop).as_ref()}.unwrap();
            let sibling = Box::new(op_extract(&perl, cv, op_sibling(o as *const unop)));
            Op::BINOP{
                opcode: oc, sibling,
                first: Box::new(op_extract(&perl, cv, op.op_first)),
                last: Box::new(op_extract(&perl, cv, op.op_last)),
            }
        },
        OPclass::OPclass_LOGOP => Op::LOGOP {opcode: oc},
        OPclass::OPclass_LISTOP => Op::LISTOP {opcode: oc},
        OPclass::OPclass_PMOP => Op::PMOP{opcode: oc},
        OPclass::OPclass_SVOP => {
            let sv = op_sv_or(o, |op| PAD_BASE_SV(CvPADLIST(cv), op.op_targ));
            Op::SVOP(oc, sv_extract(sv))
        },
        OPclass::OPclass_PADOP => {
            let op = unsafe {(o as *const padop).as_ref()}.unwrap();
            let sv = PAD_BASE_SV(CvPADLIST(cv), op.op_padix);
            Op::PADOP(oc, sv_extract(sv))
        },
        OPclass::OPclass_PVOP => Op::PVOP (oc),
        OPclass::OPclass_LOOP => Op::LOOP {opcode: oc},
        OPclass::OPclass_COP => Op::COP (oc),
        OPclass::OPclass_METHOP => {
            if (unsafe {*o}.op_flags & OPf_KIDS as u8) != 0 {
                Op::METHOP (oc, Name::Dynamic)
                
            } else {
                let sv = op_sv_or(o, |op| PAD_BASE_SV(CvPADLIST(cv), op.op_targ));
                Op::METHOP(oc, Name::Const(sv_extract(sv)))
            }
        },
        #[cfg(perlapi_ver26)]
        OPclass::OPclass_UNOP_AUX => Op::UNOP_AUX(oc /*, unsafe {(o as *const unop_aux).as_ref()}.unwrap()*/),
    }
}
