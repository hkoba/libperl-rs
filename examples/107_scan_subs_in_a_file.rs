#[cfg(perlapi_ver26)]
use std::env;

#[cfg(perlapi_ver26)]
use libperl_rs::*;

#[cfg(perlapi_ver26)]
mod eg;
#[cfg(perlapi_ver26)]
use eg::{op0::*,sv0::*,cv0::*,gv0::*};

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
        for kid in sibling_iter(o) {
            self.walk(kid, level+1);
        }
    }
}

#[cfg(perlapi_ver26)]
type Seen = std::collections::HashMap<String, bool>;

#[cfg(perlapi_ver26)]
pub struct StashWalker<'a, F, E>
where F: FnMut(*const libperl_sys::cv) -> bool,
      E: FnMut(&String, *const libperl_sys::cv)
{
    pub perl: &'a Perl,
    pub seen: Seen,
    filter: Option<&'a F>,
    emitter: &'a mut E,
}

#[cfg(perlapi_ver26)]
impl<'a, F, E> StashWalker<'a, F, E>
where F: Fn(*const libperl_sys::cv) -> bool,
      E: FnMut(&String, *const libperl_sys::cv) 
 {

    pub fn new(perl: &'a Perl,
               filter: Option<&'a F>,
               emitter: &'a mut E) -> Self {
        let mut seen = Seen::new();
        seen.insert("main".to_string(), true); // To avoid main::main::main...
        Self {
            perl: &perl, seen, filter, emitter
        }
    }

    pub fn walk(&mut self, pack: &str) {
    
        if self.seen.contains_key(pack) {return};
        self.seen.insert(pack.to_string(), true);
        
        let stash = self.perl.gv_stashpv(pack, 0);
        if stash.is_null() {return}

        // let mut packages = Vec::new();
        for (name, item) in eg::hv_iter0::HvIter::new(&self.perl, stash) {

            // ref $main::{foo} eq 'CODE'
            if let Some(Sv::CODE(cv)) = SvRV(item).map(|sv| sv_extract(sv)) {
                if (self.filter).map_or(true, |f| f(cv)) {
                    (self.emitter)(&name, cv);
                }
            }
            // ref (\$main::{foo}) eq 'GLOB'
            else if let Sv::GLOB {gv, ..} = sv_extract(item) {
                let cv = GvCV(gv);
                if (self.filter).map_or(true, |f| f(cv)) {
                    (self.emitter)(&name, cv);
                }
                if name.ends_with("::") {
                    // println!("package name = {}", name);
                    if let Some(pure) = name.get(..name.len() - 2) {
                        if !self.seen.contains_key(pure) {
                            // packages.push(String::from(pure.clone()));
                            let mut fullpack = String::from(pack);
                            fullpack.push_str("::");
                            fullpack.push_str(pure);
                            self.walk(fullpack.as_str());
                        }
                    }
                }
            }
        }
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
