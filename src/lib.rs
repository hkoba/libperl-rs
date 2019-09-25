extern crate libperl_sys;

pub mod perl;

#[cfg(test)]
mod tests {
    use super::perl::*;

    #[test]
    fn it_works() {
        let mut perl = Perl::new();
        
        let _rc = perl.parse(&["", "-e", r#"use strict; $foo"#]);
    }
}
