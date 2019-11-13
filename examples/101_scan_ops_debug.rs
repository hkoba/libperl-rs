use libperl_rs::Perl;
use libperl_sys::{op};

use std::env;

mod eg;

fn scan_ops(op: *const op) {

    for op in eg::op0::next_iter(op) {
        print!("{:?}\n", unsafe {*op});
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
