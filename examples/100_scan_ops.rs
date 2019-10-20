use libperl_rs::Perl;
use libperl_sys::{op, PL_op_name};

use std::env;
use std::ffi::CStr;

fn scan_ops(mut op: *const op) {
    while !op.is_null() {
        let ty = unsafe {(*op).op_type()};
        // let op_name = unsafe {
        //     slice::from_raw_parts(
        //         c_perl::PL_op_name as *const c_char,
        //         c_perl::PL_maxo as usize
        //     );
        // };
        print!("{:#?} {}\t{:?}\n",
               op, 
               // op_name[ty]
               unsafe {
                   CStr::from_ptr(PL_op_name[ty as usize])
               }
               .to_str().unwrap(),
               unsafe {*op},
        );
        op = unsafe {(*op).op_next as *const op};
    }
}

#[cfg(perl_useithreads)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    scan_ops(unsafe {*perl.my_perl}.Imain_start);
}

#[cfg(not(perl_useithreads))]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    scan_ops(unsafe {libperl_sys::PL_main_start});
}

// cargo run --example 100_scan_ops -- -le 'print "FOO"'

fn main() {
    my_test();
}
