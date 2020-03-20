use std::env;
use libperl_rs::*;
use libperl_sys::*;

use std::ffi::CString;

#[allow(unused_mut)]
#[cfg(perl_useithreads)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    let mut my_perl = unsafe {*(perl.my_perl)};

    // dSP
    let mut sp = my_perl.Istack_sp;

    // PUSHMARK(SP)
    unsafe {
        my_perl.Imarkstack_ptr = my_perl.Imarkstack_ptr.add(1)
    };
    if my_perl.Imarkstack_ptr == my_perl.Imarkstack_max {
        unsafe {
            Perl_markstack_grow(perl.my_perl)
        };
    }
    unsafe {
        *(my_perl.Imarkstack_ptr)
            = (sp as usize - my_perl.Istack_base as usize) as i32;
    }
    
    let subname = CString::new("foo").unwrap();

    unsafe {
        Perl_call_pv(perl.my_perl, subname.as_ptr(), G_DISCARD as i32)
    };
}

#[cfg(not(perl_useithreads))]
fn my_test() {
}

fn main() {
    my_test()
}
