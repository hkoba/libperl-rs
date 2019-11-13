use std::env;

use libperl_rs::Perl;

mod eg;

#[cfg(perlapi_ver26)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    let walker = eg::op_walker1::Walker {perl: &perl};

    walker.walk(perl.get_main_root(), 0);
}

#[cfg(not(perlapi_ver26))]
fn my_test() {}

fn main() {
    my_test();
}
