use libperl_rs::perl::Perl;
#[allow(unused)]
use libperl_sys;

// cargo run --example 000_perl_parse -- -le 'use strict; $foo'
fn main() {
    let mut perl = Perl::new();
    
    perl.parse(&["", "-e", r#"use strict; $foo"#], &[""]);

}
