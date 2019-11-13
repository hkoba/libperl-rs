use std::env;
use std::ffi::CStr;

use libperl_rs::*;

fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    // get_cvstash(perl.get_main_cv()); returns null. Why?
    
    assert_eq!(perl.gv_stashpv("main", 0), perl.get_defstash());
    
    let stash = perl.get_defstash();
    
    unsafe {libperl_sys::Perl_hv_iterinit(perl.my_perl, stash)};
    
    let mut he = unsafe {libperl_sys::Perl_hv_iternext(perl.my_perl, stash)};
    while !he.is_null() {
        let mut nlen: i32 = 0;
        let name = unsafe {CStr::from_ptr(unsafe {libperl_sys::Perl_hv_iterkey(perl.my_perl, he, &mut nlen)})};
        let val = unsafe {libperl_sys::Perl_hv_iterval(perl.my_perl, stash, he)};
        println!("name = {:?} value = {:?}", name, val);
        he = unsafe {libperl_sys::Perl_hv_iternext(perl.my_perl, stash)};
    }
}

#[cfg(not(perlapi_ver26))]
fn my_test() {}

fn main() {
    my_test();
}
