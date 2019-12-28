#[cfg(perlapi_ver26)]
use std::env;

#[cfg(perlapi_ver26)]
use libperl_rs::*;

#[cfg(perlapi_ver26)]
mod eg;
#[cfg(perlapi_ver26)]
use eg::{op1::*,sv0::*,cv0::*,stash_walker0::*};

#[cfg(perlapi_ver26)]
pub struct OpWalker<'a> {
    pub perl: &'a Perl,
    pub cv: *const libperl_sys::cv,
}

#[cfg(perlapi_ver26)]
impl<'a> OpWalker<'a> {
    pub fn walk(&'a self, o: *const op, level: isize) {
        if o.is_null() {return}
        print!("{}", "  ".repeat(level as usize));
        let ox = op_extract(&self.perl, self.cv, o);
        println!("{:?} {:?}", op_name(o), ox);
        // for kid in sibling_iter(o) {
        //     self.walk(kid, level+1);
        // }
    }
}

#[cfg(perlapi_ver26)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    let main_file = sv_extract_pv(perl.get_sv("0", 0)).unwrap();
    println!("$0 = {:?}", main_file);
    
    let filter = |cv| CvFILE(cv).map_or(false, |s| s == main_file);

    let mut emitter = |name: &String, cv: *const libperl_sys::cv| {
        let walker = OpWalker {perl: &perl, cv};
        println!("sub {:?}", name);
        walker.walk(CvROOT(cv), 0);
        println!("");
    };

    let mut nswalker = StashWalker::new(&perl, Some(&filter), &mut emitter);

    nswalker.walk("");
}

#[cfg(not(perlapi_ver26))]
fn my_test() {
    println!("Requires perl >= 5.26");
}

fn main() {
    my_test();
}
