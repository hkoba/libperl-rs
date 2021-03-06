#[cfg(perlapi_ver26)]
use std::env;

#[cfg(perlapi_ver26)]
use libperl_rs::*;

#[cfg(perlapi_ver26)]
mod eg;
#[cfg(perlapi_ver26)]
use eg::{op0::*,sv0::*,cv0::*,gv0::*};

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
    
    let main_file = sv_extract_pv(perl.get_sv("0", 0)).unwrap();
    println!("$0 = {:?}", main_file);

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
        else if let Sv::GLOB {gv, ..} = sv_extract(item) {
            let cv = GvCV(gv);
            if let Some(file) = CvFILE(cv) {
                if file == main_file {
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
    println!("Requires perl >= 5.26");
}

fn main() {
    my_test();
}
