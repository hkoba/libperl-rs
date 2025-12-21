pub mod perl_core;
pub use perl_core::*;

pub mod conv_opcode;

pub mod sigdb;

use std::ffi::CStr;

// use std::os::raw::{c_char, c_int /*, c_void, c_schar*/};

fn core_op_name(o: &op) -> Option<String> {
    let ty = o.op_type();
    if (ty as usize) < unsafe {PL_op_name.len()} {
        let op_name = unsafe {CStr::from_ptr(PL_op_name[ty as usize])};
        Some(String::from(op_name.to_str().unwrap()))
    } else {
        None
    }
}

impl std::fmt::Display for op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ {:?}={:#?} {:?} }}"
               , core_op_name(&self)
               , (self as *const op)
               , self)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let perl = unsafe { super::perl_alloc() };
        unsafe {
            super::perl_construct(perl);
        };
    }

    #[test]
    fn sigdb_lookup() {
        use super::sigdb::{FN_BY_NAME, FUNCS};

        // Test that FN_BY_NAME lookup works
        if let Some(id) = FN_BY_NAME.get("Perl_sv_isbool") {
            let sig = &FUNCS[id.0 as usize];
            assert_eq!(sig.name, "Perl_sv_isbool");
            assert!(!sig.ret.is_empty());
        }

        // Test perl_alloc
        let id = FN_BY_NAME.get("perl_alloc").expect("perl_alloc should exist");
        let sig = &FUNCS[id.0 as usize];
        assert_eq!(sig.name, "perl_alloc");
        assert!(sig.ret.contains("PerlInterpreter"));
    }
}
