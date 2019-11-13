use libperl_rs::Perl;

use libperl_sys::*;

use super::op0::*;

pub struct Walker<'a> {
    pub perl: &'a Perl,
}

impl<'a> Walker<'a> {


    #[cfg(perlapi_ver26)]
    pub fn op_extract(&'a self, o: *const op) -> Op {
        let cls = self.perl.op_class(o);
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

    #[cfg(perlapi_ver26)]
    pub fn walk(&'a self, o: *const op, level: isize) {
        let mut kid = op_first(o);
        while !kid.is_null() {
            self.walk(kid, level+1);
            kid = op_sibling(kid as *const unop);
        }
        print!("{}", "  ".repeat(level as usize));
        println!("{:?} {:?}", op_name(o), self.op_extract(o));
    }
}
