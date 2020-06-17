#[cfg(perlapi_ver26)]
use std::env;

#[cfg(perlapi_ver26)]
use libperl_rs::*;

#[cfg(perlapi_ver26)]
mod eg;
#[cfg(perlapi_ver26)]
use eg::{op1::*,sv0::*,cv0::*,stash_walker0::*};

#[cfg(perlapi_ver26)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    let op_extractor = OpExtractor::new(&perl);

    let main_file = sv_extract_pv(perl.get_sv("0", 0)).unwrap();
    println!("$0 = {:?}", main_file);
    
    let filter = |cv| CvFILE(cv).map_or(false, |s| s == main_file);

    let mut emitter = |name: &String, cv: *const libperl_sys::cv| {
        println!("sub {:?}", name);
        println!("{:#?}", op_extractor.extract(cv, CvROOT(cv)));
        println!("");
    };

    let mut nswalker = StashWalker::new(&perl, Some(&filter), &mut emitter);

    nswalker.walk("");
    
    let main_cv = perl.get_main_cv();

    println!("#main_cv");
    // XXX: CvROOT(main_cv) doesn't work here.
    println!("{:#?}", op_extractor.extract(main_cv, perl.get_main_root()));
    println!("");
}

#[cfg(not(perlapi_ver26))]
fn my_test() {
    println!("Requires perl >= 5.26");
}

fn main() {
    my_test();
}
