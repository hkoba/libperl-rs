#![allow(non_snake_case)]

use super::libperl_sys::*;

use std::ptr;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::env;

#[cfg(perl_useithreads)]
#[macro_export]
macro_rules! perl_api {
    ($name:ident ($my_perl:expr $(, $arg:expr)*) $(as $t:ty)*) => {
        $name($my_perl, $($arg),*) $(as $t)*
    }
}

#[cfg(not(perl_useithreads))]
#[macro_export]
macro_rules! perl_api {
    ($name:ident ($my_perl:expr $(, $arg:expr)*) $(as $t:ty)*) => {
        $name($($arg),*) $(as $t)*
    }
}

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

extern "C" {
    #[cfg(perl_useithreads)]
    fn boot_DynaLoader(perl: *mut PerlInterpreter, cv: *mut CV);
    #[cfg(not(perl_useithreads))]
    fn boot_DynaLoader(cv: *mut CV);
}

#[allow(non_camel_case_types)]
#[cfg(perl_useithreads)]
type xsinit_type = extern "C" fn(*mut PerlInterpreter) -> ();

#[allow(non_camel_case_types)]
#[cfg(not(perl_useithreads))]
type xsinit_type = extern "C" fn() -> ();

#[cfg(perl_useithreads)]
pub fn newXS(perl: *mut PerlInterpreter, name: &str, xsub: XSUBADDR_t, filename: &str) -> *mut CV {
    let name = CString::new(name).unwrap();
    let filename = CString::new(filename).unwrap();
    unsafe {Perl_newXS(perl, name.as_ptr(), xsub, filename.as_ptr())}
}

#[cfg(not(perl_useithreads))]
pub fn newXS(name: &str, xsub: XSUBADDR_t, filename: &str) -> *mut CV {
    let name = CString::new(name).unwrap();
    let filename = CString::new(filename).unwrap();
    unsafe {Perl_newXS(name.as_ptr(), xsub, filename.as_ptr())}
}

#[cfg(perl_useithreads)]
pub extern "C" fn xs_init(perl: *mut PerlInterpreter) {
    newXS(perl, "DynaLoader::boot_DynaLoader", Some(boot_DynaLoader), file!());
}

#[cfg(not(perl_useithreads))]
pub extern "C" fn xs_init() {
    newXS("DynaLoader::boot_DynaLoader", Some(boot_DynaLoader), file!());
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
                Some(xs_init as xsinit_type),
                self.args.len() as c_int,
                make_argv_from_vec(&self.args)
                    .as_ptr() as *mut *mut c_char,
                ensure_terminating_null(make_argv_from_vec(&self.env))
                    .as_ptr() as *mut *mut c_char,
            )
        }
    }
    
    pub fn hv_iterinit(&self, hv: *mut HV) -> i32 {
        unsafe {perl_api!{Perl_hv_iterinit(self.my_perl, hv)}}
    }
    
    pub fn hv_iternext(&self, hv: *mut HV) -> *mut HE {
        unsafe {perl_api!{Perl_hv_iternext_flags(self.my_perl, hv, 0)}}
    }
    
    pub fn hv_iterkey(&self, he: *mut HE) -> String {
        let (name, nlen) = self._hv_iterkey(he);
        let slice = unsafe {std::slice::from_raw_parts(name, nlen)};
        String::from_utf8(slice.to_vec()).unwrap()
    }

    pub fn _hv_iterkey(&self, he: *mut HE) -> (*const u8, usize) {
        let mut nlen: i32 = 0;
        let name = unsafe {perl_api!{Perl_hv_iterkey(self.my_perl, he, &mut nlen) as *const u8}};
        (name, nlen as usize)
    }

    pub fn hv_iterval<'a>(&self, hv: *mut HV, he: *mut HE) -> *mut SV {
        unsafe {perl_api!{Perl_hv_iterval(self.my_perl, hv, he)}}
    }

    #[cfg(perl_useithreads)]
    pub fn get_defstash(&self) -> *mut HV {
        unsafe {*self.my_perl}.Idefstash
    }
    #[cfg(not(perl_useithreads))]
    pub fn get_defstash(&self) -> *mut HV {
        unsafe {libperl_sys::PL_defstash}
    }

    pub fn gv_stashpv(&self, name: &str, flags: i32) -> *mut HV {
        let name = CString::new(name).unwrap();
        unsafe {perl_api!{Perl_gv_stashpv(self.my_perl, name.as_ptr(), flags)}}
    }

    #[cfg(perl_useithreads)]
    pub fn get_main_root(&self) -> *const op {
        unsafe {*self.my_perl}.Imain_root
    }

    #[cfg(not(perl_useithreads))]
    pub fn get_main_root(&self) -> *const op {
        unsafe {libperl_sys::PL_main_root}
    }

    #[cfg(perl_useithreads)]
    pub fn get_main_cv(&self) -> *const cv {
        unsafe {*self.my_perl}.Imain_cv
    }

    #[cfg(not(perl_useithreads))]
    pub fn get_main_cv(&self) -> *const cv {
        unsafe {libperl_sys::PL_main_cv}
    }

    #[cfg(all(perlapi_ver26))]
    pub fn op_class(&self, o: *const OP) -> OPclass {
        unsafe {perl_api!{Perl_op_class(self.my_perl, o)}}
    }
    
    pub fn get_sv(&self, name: &str, flags: i32) -> *mut SV {
        let name = CString::new(name).unwrap();
        unsafe {perl_api!{Perl_get_sv(self.my_perl, name.as_ptr(), flags)}}
    }
    
    pub fn str2svpv_flags(&self, buffer: &str, flags: u32) -> *mut SV {
        let cstr = CString::new(buffer).unwrap();
        unsafe {
            perl_api!{Perl_newSVpvn_flags(
                self.my_perl,
                cstr.as_ptr(),
                buffer.len(),
                flags
            )}
        }
    }
}

pub fn get_cvstash(cv: *const CV) -> *mut HV {
    let xpvcv = unsafe {*cv}.sv_any;
    unsafe {*xpvcv}.xcv_stash
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
