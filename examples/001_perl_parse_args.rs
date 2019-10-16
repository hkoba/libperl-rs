use libperl_rs::Perl;
#[allow(unused)]
use libperl_sys;

use std::env;

fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
}

// cargo run --example 001_perl_parse_args -- -le 'use strict; $bar'

fn main() {
    my_test();
}
