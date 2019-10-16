use libperl_rs::Perl;
use libperl_sys::{op};

use std::env;

fn scan_ops(mut op: *const op) {
    while !op.is_null() {
        print!("{}\n", unsafe {*op});
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


// cargo run --example 101_scan_ops_debug -- -le 'print "FOO"'

fn main() {
    my_test();
}
