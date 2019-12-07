use std::env;

use libperl_sys::op;
use libperl_rs::Perl;

mod eg;
use eg::op0::*;

#[cfg(perlapi_ver26)]
pub struct Walker<'a> {
    pub perl: &'a Perl,
    pub cv: *const libperl_sys::cv,
}

#[cfg(perlapi_ver26)]
impl<'a> Walker<'a> {
    pub fn walk(&'a self, o: *const op, level: isize) {
        for kid in sibling_iter(o) {
            self.walk(kid, level+1);
        }
        print!("{}", "  ".repeat(level as usize));
        println!("{:?} {:?}", op_name(o), op_extract(&self.perl, self.cv, o));
    }
}

#[cfg(perlapi_ver26)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    let walker = Walker {perl: &perl, cv: perl.get_main_cv()};

    walker.walk(perl.get_main_root(), 0);
}

#[cfg(not(perlapi_ver26))]
fn my_test() {
    println!("Requires perl >= 5.26");
}

fn main() {
    my_test();
}
