use libperl_rs::perl::Perl;
#[allow(unused)]
use libperl_sys;

use std::env;

fn test() {
    let mut perl = Perl::new();
    perl.parse_args(env::args(), &[]);
}

// cargo run --example 001_perl_parse_args -- -le 'use strict; $bar'

fn main() {
    test();
}
