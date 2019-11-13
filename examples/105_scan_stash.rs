use std::env;

use libperl_rs::*;

mod eg;

fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    // get_cvstash(perl.get_main_cv()); returns null. Why?
    
    let stash = perl.get_defstash();
    
    assert_eq!(perl.gv_stashpv("main", 0), stash);
    
    for (name, value) in eg::hv_iter0::HvIter::new(&perl, stash) {
        println!("name = {:?} value = {:?}", name, value);        
    }
}

fn main() {
    my_test();
}
