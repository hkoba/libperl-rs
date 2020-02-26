use std::env;
use libperl_rs::*;
use libperl_sys::*;

use std::ffi::CString;

fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    // dSP
    let mut sp = perl.my_perl.Istack_sp;
    // PUSHMARK(SP)
    perl.my_perl.Imarkstack_ptr = perl.my_perl.Imarkstack_ptr.add(1);
    if perl.my_perl.Imarkstack_ptr == perl.my_perl.Imarkstack_max {
        markstack_grow(perl.my_perl);
    }
    unsafe {
        *(perl.my_perl.Imarkstack_ptr)
            = (sp as usize - perl.my_perl.Istack_base as usize) as i32;
    }
    
    let subname = CString::new("foo");

    call_sv(perl.my_perl, subname.as_ptr(), G_DISCARD);
}

fn main() {
    my_test()
}
