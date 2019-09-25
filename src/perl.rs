use super::libperl_sys::*;

use std::ptr;
use std::ffi::{CString};
use std::os::raw::{c_char, c_int};

pub struct Perl {
    debug: bool,
    args: Vec<CString>,
    env: Vec<CString>,
    pub my_perl: *mut PerlInterpreter,
}

impl Drop for Perl {
    fn drop(&mut self) {
        if self.debug {
            println!("destructuring my perl");
        }
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
            debug: false,
        }
    }
    
    pub fn parse(&mut self, args: &[&str]) -> i32 {
        self.args = args.iter().map(|&arg| CString::new(arg).unwrap() )
            .collect::<Vec<CString>>();
        let c_args = self.args.iter().map(|arg| arg.as_ptr() as *mut c_char)
            .collect::<Vec<*mut c_char>>();
        // let c_env = self.env.iter().map(|arg| arg.as_ptr()).collect::<Vec<*const c_char>>();

        unsafe {
            perl_parse(
                self.my_perl,
                None,
                c_args.len() as c_int,
                c_args.as_ptr() as *mut *mut c_char,
                ptr::null_mut(),
            )
        }
    }
}
