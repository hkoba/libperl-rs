#![allow(non_snake_case)]
pub use libperl_sys::op;

use if_chain::if_chain;

use libperl_sys::*;
use libperl_rs::Perl;

use super::sv0::{Sv, VarName, sv_extract};
use super::pad0::*;
use super::op0::{op_sibling, op_sv_or, Name};
pub use super::op0::op_name;

use typed_arena::Arena;

pub struct OpExtractor<'a> {
    perl: &'a Perl,
    ops: Arena<Op<'a>>,
}

#[derive(Debug, Clone)]
pub struct PadNameType {
    name: Option<String>,
    typ: Option<String>,
}

struct OpcodeWrap<'a> (&'a opcode);

impl<'a> std::fmt::Debug for OpcodeWrap<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("opcode::")?;
        self.0.fmt(f)
    }
}

#[allow(non_camel_case_types)]
pub enum Op<'a> {
    NULL,
    OP (opcode, *const op, Option<PadNameType>, &'a Op<'a>),
    UNOP (opcode, *const unop, &'a Op<'a>, &'a Op<'a>),
    BINOP (opcode, *const binop, &'a Op<'a>, &'a Op<'a>),
    LOGOP (opcode, *const logop, &'a Op<'a>, &'a Op<'a>),
    LISTOP (opcode, *const listop, &'a Op<'a>, &'a Op<'a>), // , last: &'a Op<'a>
    PMOP {opcode: opcode// , sibling: &'a Op<'a>, first: &'a Op<'a>, last: &'a Op<'a>
    },
    SVOP (opcode, Sv, &'a Op<'a>),
    PADOP (opcode, *const padop, Sv),
    PVOP (opcode/*, &'a pvop*/),
    LOOP (opcode, &'a Op<'a> //, first: &'a Op<'a>, last: &'a Op<'a>, redoop: &'a Op<'a>, next: &'a Op<'a>, last: &'a Op<'a>
    ),
    COP (opcode, &'a Op<'a>),
    METHOP(opcode, Name),
    // {opcode: opcode, sibling: &'a Op<'a>, name: Name},
    #[cfg(perlapi_ver26)]
    UNOP_AUX (opcode, &'a Op<'a>, &'a Op<'a>),
}

impl<'a> std::fmt::Debug for  Op<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Op::NULL => { f.debug_tuple("Op::NULL").finish() },
            Op::OP(oc, _, padname, sibling) => {
                f.debug_tuple("Op::OP")
                    .field(&OpcodeWrap(oc))
                    .field(&VarName("_"))
                    .field(&padname)
                    .field(&sibling)
                    .finish()
            },
            Op::UNOP(oc, _, first, sibling) => {
                f.debug_tuple("Op::UNOP")
                    .field(&OpcodeWrap(oc))
                    .field(&VarName("_"))
                    .field(&first)
                    .field(&sibling)
                    .finish()
            },
            Op::BINOP(oc, _, first, sibling) => {
                f.debug_tuple("Op::BINOP")
                    .field(&OpcodeWrap(oc))
                    .field(&VarName("_"))
                    .field(&first)
                    .field(&sibling)
                    .finish()
            },
            Op::LOGOP(oc, _, first, sibling) => {
                f.debug_tuple("Op::LOGOP")
                    .field(&OpcodeWrap(oc))
                    .field(&VarName("_"))
                    .field(&first)
                    .field(&sibling)
                    .finish()
            },
            Op::LISTOP(oc, _, first, sibling) => {
                f.debug_tuple("Op::LISTOP")
                    .field(&OpcodeWrap(oc))
                    .field(&VarName("_"))
                    .field(&first)
                    .field(&sibling)
                    .finish()
            },
            Op::PMOP {opcode: oc} => {
                f.debug_struct("Op::PMOP")
                    .field("opcode", &OpcodeWrap(oc))
                    // XXX
                    .finish()
            },
            Op::SVOP(oc, sv, sibling) => {
                f.debug_tuple("Op::SVOP")
                    .field(&OpcodeWrap(oc))
                    .field(&sv)
                    .field(&sibling)
                    .finish()
            },
            Op::PADOP(oc, _, sibling) => {
                f.debug_tuple("Op::PADOP")
                    .field(&OpcodeWrap(oc))
                    .field(&VarName("_"))
                    .field(&sibling)
                    .finish()
            },
            Op::PVOP(oc) => {
                f.debug_tuple("Op::PVOP")
                    .field(&OpcodeWrap(oc))
                    .finish()
            },
            Op::LOOP(oc, sibling) => {
                f.debug_tuple("Op::LOOP")
                    .field(&OpcodeWrap(oc))
                    .field(&sibling)
                    .finish()
            },
            Op::COP(oc, sibling) => {
                f.debug_tuple("Op::COP")
                    .field(&OpcodeWrap(oc))
                    .field(sibling)
                    .finish()
            },
            Op::METHOP(oc, name) => {
                f.debug_tuple("Op::METHOP")
                    .field(&OpcodeWrap(oc))
                    .field(&name)
                    .finish()
            },
            #[cfg(perlapi_ver26)]
            Op::UNOP_AUX(oc, first, sibling) => {
                f.debug_tuple("Op::UNOP_AUX")
                    .field(&OpcodeWrap(oc))
                    .field(&first)
                    .field(&sibling)
                    .finish()
            },
        }
    }
}

#[cfg(perlapi_ver26)]
impl<'a> OpExtractor<'a> {
    
    pub fn new(perl: &'a Perl) -> Self {
        Self {perl, ops: Arena::new()}
    }

    pub fn extract(&self, cv: *const cv, o: *const op) -> &'a Op {
        if o.is_null() {
            return self.ops.alloc(Op::NULL)
        }
        let cls = self.perl.op_class(o);
        let oc = unsafe {
            let ty = (*o).op_type();
            *(&ty as *const u32 as *const opcode)
        };

        let eo = match cls {
            OPclass::OPclass_NULL => Op::NULL,
            OPclass::OPclass_BASEOP => {
                let op = unsafe {o.as_ref().unwrap()};
                let sibling = self.extract(cv, op_sibling(o as *const unop));
                if_chain! {
                    if let Some(pl) = cv_padnamelist(cv);
                    if let Some(padname) = padnamelist_nth(pl, op.op_targ as usize);
                    then {
                        Op::OP (
                            oc, o,
                            Some(PadNameType {
                                name: PadnamePV(padname), typ: PadnameTYPE(padname)
                            }),
                            sibling
                        )
                    } else {
                        Op::OP (oc, o, None, sibling)
                    }
                }
            },
            OPclass::OPclass_UNOP => {
                let op = unsafe {(o as *const unop).as_ref()}.unwrap();
                Op::UNOP (
                    oc, op,
                    self.extract(cv, op.op_first),
                    self.extract(cv, op_sibling(o as *const unop)),
                )
            },
            OPclass::OPclass_BINOP => {
                let op = unsafe {(o as *const binop).as_ref()}.unwrap();
                Op::BINOP (
                    oc, op,
                    self.extract(cv, op.op_first),
                    self.extract(cv, op_sibling(o as *const unop)),
                )
            },
            OPclass::OPclass_LOGOP => {
                let op = unsafe {(o as *const logop).as_ref()}.unwrap();
                Op::LOGOP (
                    oc, op,
                    self.extract(cv, op.op_first),
                    self.extract(cv, op_sibling(o as *const unop)),
                )
            },
            OPclass::OPclass_LISTOP => {
                let op = unsafe {(o as *const listop).as_ref()}.unwrap();
                Op::LISTOP (
                    oc, op,
                    self.extract(cv, op.op_first),
                    self.extract(cv, op_sibling(o as *const unop)),
                )
            },
            // XXX
            OPclass::OPclass_PMOP => Op::PMOP {opcode: oc},
            OPclass::OPclass_SVOP => {
                let sv = op_sv_or(o, |op| PAD_BASE_SV(CvPADLIST(cv), op.op_targ));
                Op::SVOP (
                    oc, sv_extract(sv),
                    self.extract(cv, op_sibling(o as *const unop)),
                )
            },
            OPclass::OPclass_PADOP => {
                let op = unsafe {(o as *const padop).as_ref()}.unwrap();
                let sv = PAD_BASE_SV(CvPADLIST(cv), op.op_padix);
                Op::PADOP(oc, op, sv_extract(sv))
            },
            // XXX
            OPclass::OPclass_PVOP => Op::PVOP (oc),
            // XXX
            OPclass::OPclass_LOOP => {
                Op::LOOP (
                    oc,
                    self.extract(cv, op_sibling(o as *const unop)),
                )
            },
            OPclass::OPclass_COP => {
                // let op = unsafe {(o as *const cop).as_ref()}.unwrap();
                Op::COP (
                    oc,
                    self.extract(cv, op_sibling(o as *const unop)),
                )
            },
            // XXX
            OPclass::OPclass_METHOP => {
                if (unsafe {*o}.op_flags & OPf_KIDS as u8) != 0 {
                    Op::METHOP (oc, Name::Dynamic)
                        
                } else {
                    let sv = op_sv_or(o, |op| PAD_BASE_SV(CvPADLIST(cv), op.op_targ));
                    Op::METHOP(oc, Name::Const(sv_extract(sv)))
                }
            },
            #[cfg(perlapi_ver26)]
            OPclass::OPclass_UNOP_AUX => {
                let op = unsafe {(o as *const unop_aux).as_ref()}.unwrap();
                Op::UNOP_AUX (
                    oc,
                    self.extract(cv, op.op_first),
                    self.extract(cv, op_sibling(o as *const unop)),
                )
            }
        };
        
        self.ops.alloc(eo)
    }
}
