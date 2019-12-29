extern crate libperl_sys;

#[allow(unused_imports)]
#[macro_use]
extern crate if_chain; // For OpExtractor

pub mod perl;
pub use perl::*;

#[cfg(test)]
mod tests {
    use super::perl::*;

    #[test]
    fn it_works() {
        let mut perl = Perl::new();
        
        // Below is expected to generate an error like following:
        // Global symbol "$foo" requires explicit package name (did you forget to declare "my $foo"?) at -e line 1.
        let _rc = perl.parse(&["", "-e", r#"use strict; $foo"#], &[""]);
    }
}
