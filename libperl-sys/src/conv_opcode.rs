use std::convert::TryFrom;

use crate::perl_core::{op, opcode};

impl TryFrom<*const op> for opcode {
    type Error = &'static str;

    fn try_from(op: *const op) -> Result<Self, Self::Error> {
        if op.is_null() {
            return Err("Null OP*")
        }
        opcode::try_from(unsafe {
            (*op).op_type()
        })
    }
}

impl TryFrom<u32> for opcode {
    type Error = &'static str;

    fn try_from(oc: u32) -> Result<Self, Self::Error> {
        if oc <= opcode::OP_max as u32 {
            let e = unsafe {std::mem::transmute(oc)};
            Ok(e)
        } else {
            Err("Invalid opcode")
        }
    }
}

impl TryFrom<u16> for opcode {
    type Error = &'static str;

    fn try_from(oc: u16) -> Result<Self, Self::Error> {
        if oc <= opcode::OP_max as u16 {
            let e = unsafe {std::mem::transmute(oc as u32)};
            Ok(e)
        } else {
            Err("Invalid opcode")
        }
    }
}
