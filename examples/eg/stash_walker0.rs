use libperl_rs::Perl;
use super::*;
use super::{sv0::*,gv0::*};

#[cfg(perlapi_ver26)]
type Seen = std::collections::HashMap<String, bool>;

#[cfg(perlapi_ver26)]
pub struct StashWalker<'a, F, E>
where F: Fn(*const libperl_sys::cv) -> bool,
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

        for (name, item) in hv_iter0::HvIter::new(&self.perl, stash) {

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
