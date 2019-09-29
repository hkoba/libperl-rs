use super::libperl_sys::*;

use std::ptr;
use std::ffi::{CString};
use std::os::raw::{c_char, c_int};
use std::env;

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
    
    pub fn parse(&mut self, args: &[&str], envp: &[&str]) -> i32 {
        self.args = args.iter().map(|&arg| CString::new(arg).unwrap())
            .collect::<Vec<CString>>();
        self.env = envp.iter().map(|&arg| CString::new(arg).unwrap())
            .collect::<Vec<CString>>();
        
        self.perl_parse_1()
    }
    
    pub fn parse_env_args(&mut self, args: env::Args, envp: env::Vars) -> i32 {
        self.args = args.map(|arg| CString::new(arg).unwrap())
            .collect::<Vec<CString>>();
        self.env = envp.map(| (key, value) | CString::new(
            String::from(&[key, value].join("="))
        ).unwrap()).collect::<Vec<CString>>();
        
        self.perl_parse_1()
    }

    fn perl_parse_1(&mut self) -> i32 {
        unsafe {
            perl_parse(
                self.my_perl,
                None,
                self.args.len() as c_int,
                make_argv_from_vec(&self.args)
                    .as_ptr() as *mut *mut c_char,
                ensure_terminating_null(make_argv_from_vec(&self.env))
                    .as_ptr() as *mut *mut c_char,
            )
        }
    }
}

pub fn make_argv_from_vec(args: &Vec<CString>) -> Vec<*mut c_char> {
    args.iter().map(|arg| arg.as_ptr() as *mut c_char)
        .collect::<Vec<*mut c_char>>()
}

pub fn ensure_terminating_null(mut args: Vec<*mut c_char>) -> Vec<*mut c_char> {
    if args.len() == 0 || args.last() != None {
        args.push(ptr::null_mut());
    }
    args
}
