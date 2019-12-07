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
        if o.is_null() {return}
        for kid in sibling_iter(o) {
            self.walk(kid, level+1);
        }
        print!("{}", "  ".repeat(level as usize));
        let ox = op_extract(&self.perl, self.cv, o);
        println!("{:?} {:?}", op_name(o), ox);
    }
}

#[cfg(perlapi_ver26)]
fn my_test() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());
    
    let mut seen = Seen::new();
    seen.insert("main".to_string(), true); // To avoid main::main::main...
    stash_subs(&perl, "", &mut seen);
}

type Seen = std::collections::HashMap<String, bool>;

fn stash_subs(perl: &Perl, pack: &str, seen: &mut Seen) {
    println!("pack = {}", pack);

    if seen.contains_key(pack) {return};
    seen.insert(pack.to_string(), true);
    
    let stash = perl.gv_stashpv(pack, 0);
    if stash.is_null() {return}

    let emitter = |name: &String, cv: *const libperl_sys::cv| {
        let walker = Walker {perl: &perl, cv};
        println!("sub {:?} file {:?}", name, CvFILE(cv));
        walker.walk(CvROOT(cv), 0);
        println!("");
    };

    // let mut packages = Vec::new();
    for (name, item) in eg::hv_iter0::HvIter::new(&perl, stash) {

        if name == "strerror2lintresult" || name == "process_request" {
            println!("Found target! name = {}", name);
        }

        // ref $main::{foo} eq 'CODE'
        if let Some(Sv::CODE(cv)) = SvRV(item).map(|sv| sv_extract(sv)) {
            emitter(&name, cv)
        }
        // ref (\$main::{foo}) eq 'GLOB'
        else if let Sv::GLOB {gv, ..} = sv_extract(item) {
            let cv = GvCV(gv);
            if let Some(file) = CvFILE(cv) {
                emitter(&name, cv);
            }
            if name.ends_with("::") {
                println!("package name = {}", name);
                if let Some(pure) = name.get(..name.len() - 2) {
                    if !seen.contains_key(pure) {
                        // packages.push(String::from(pure.clone()));
                        let mut fullpack = String::from(pack);
                        fullpack.push_str("::");
                        fullpack.push_str(pure);
                        stash_subs(perl, fullpack.as_str(), seen);
                    }
                }
            }
        }
    }
    
    // for pkg in packages {
    //     stash_subs(perl, pkg.as_str(), seen);
    // }
}

#[cfg(not(perlapi_ver26))]
fn my_test() {
}

fn main() {
    my_test();
}
