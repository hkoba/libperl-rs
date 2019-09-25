use super::libperl_sys::*;

use std::ffi::{CString};

pub struct Perl {
    args: Vec<CString>,
    env: Vec<CString>,
    pub my_perl: *mut PerlInterpreter,
}

impl Drop for Perl {
    fn drop(&mut self) {
        println!("destructuring my perl");
        unsafe { perl_destruct(self.my_perl) };
    }
}

impl Perl {

    pub fn new() -> Perl {
        let perl = unsafe {perl_alloc()};
        unsafe {perl_construct(perl)};
        return Perl {
            args: Vec::new(),
            env: Vec::new(),
            my_perl: perl,
        }
    }
}
