use std::env;

use libperl_rs::*;

mod eg;
use eg::op0::*;
use eg::sv0::*;
use eg::cv0::*;
use eg::gv0::*;

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
    
    // get_cvstash(perl.get_main_cv()); returns null. Why?
    
    let stash = perl.get_defstash();
    
    assert_eq!(perl.gv_stashpv("main", 0), stash);
    
    println!("main file = {:?}", CvFILE(perl.get_main_cv()));

    let emitter = |name: &String, cv: *const libperl_sys::cv| {
        let walker = Walker {perl: &perl, cv};
        println!("sub {:?} file {:?}", name, CvFILE(cv));
        walker.walk(CvROOT(cv), 0);
        println!("");
    };

    for (name, item) in eg::hv_iter0::HvIter::new(&perl, stash) {

        // ref $main::{foo} eq 'CODE'
        if let Some(Sv::CODE(cv)) = SvRV(item).map(|sv| sv_extract(sv)) {
            emitter(&name, cv)
        }
        // ref (\$main::{foo}) eq 'GLOB'
        else if let Sv::GLOB(gv, _, _) = sv_extract(item) {
            let cv = GvCV(gv);
            if let Some(file) = CvFILE(cv) {
                if file == "-e" {
                    emitter(&name, cv);
                } else {
                    println!("name = {:?} from file {:?}", name, file);
                }
            }
        }
        
    }
}

#[cfg(not(perlapi_ver26))]
fn my_test() {
}

fn main() {
    my_test();
}
