use libperl_rs::Perl;
#[allow(unused)]
use libperl_sys;

// cargo run --example 000_perl_parse -- -le 'use strict; $foo'
// This will print an error like following:
//
//   Global symbol "$foo" requires explicit package name (did you forget to declare "my $foo"?) at -e line 1.
//   Execution of -e aborted due to compilation errors.


fn main() {
    let mut perl = Perl::new();
    
    perl.parse(&["", "-e", r#"use strict; $foo"#], &[]);
}
