//! Stash (package symbol table) traversal — enumerate the subs of a
//! package tree the way `%Foo::` looks in Perl. Step 2's north-star
//! walker (`docs/plan/README.md` §4 Step 2.3), rebuilt from
//! libperl-proto0's `eg/stash_walker0.rs` / examples `105`-`107` on
//! the new newtype layer.
//!
//! ```no_run
//! # use libperl_rs::*;
//! # let mut perl = Perl::new();
//! let mut walker = StashWalker::new(&perl);
//! walker.walk("main", &mut |e| {
//!     println!("{}::{} from {:?}", e.package, e.name, e.cv.file());
//! });
//! ```

use std::collections::HashSet;

use crate::{Cv, Gv, Perl, SvKind};

/// One named sub found in a stash: `package` is the normalized
/// package path (`"main"`, `"Foo::Bar"`), `name` the symbol name
/// inside it. `gv` is the glob the CV hangs off, or `None` when the
/// stash slot held a bare code reference instead of a full glob (the
/// sub-ref-in-stash optimisation).
pub struct SubEntry {
    pub package: String,
    pub name: String,
    pub cv: Cv,
    pub gv: Option<Gv>,
}

/// Recursive stash walker with cycle protection. Reusable across
/// multiple [`walk`](StashWalker::walk) calls — the seen-set persists,
/// so each package is visited at most once per walker.
pub struct StashWalker<'p> {
    perl: &'p Perl,
    seen: HashSet<String>,
}

impl<'p> StashWalker<'p> {
    pub fn new(perl: &'p Perl) -> Self {
        StashWalker {
            perl,
            seen: HashSet::new(),
        }
    }

    /// Walk the package `pack` (typically `"main"`) and, recursively,
    /// every sub-package reachable from it, calling `emit` for each
    /// named sub. Symbols other than subs (scalars, arrays, ...) are
    /// skipped.
    pub fn walk(&mut self, pack: &str, emit: &mut dyn FnMut(&SubEntry)) {
        if !self.seen.insert(pack.to_string()) {
            return;
        }
        let Some(stash) = self.perl.gv_stashpv(pack, 0) else {
            return;
        };
        for (key, val) in stash.iter(self.perl) {
            let name = String::from_utf8_lossy(key).into_owned();
            match val.kind() {
                // `ref $Foo::{bar} eq 'CODE'` — the slot holds a bare
                // sub ref (constant / stub optimisation), no glob.
                SvKind::Ref(target) => {
                    if let SvKind::Code(cv) = target.kind() {
                        emit(&SubEntry {
                            package: pack.to_string(),
                            name,
                            cv,
                            gv: None,
                        });
                    }
                }
                SvKind::Glob(gv) => {
                    if let Some(cv) = gv.cv() {
                        emit(&SubEntry {
                            package: pack.to_string(),
                            name: name.clone(),
                            cv,
                            gv: Some(gv),
                        });
                    }
                    // A trailing-`::` glob is a sub-package: recurse.
                    if let Some(base) = name.strip_suffix("::") {
                        if !base.is_empty() {
                            let child = if pack == "main" {
                                base.to_string()
                            } else {
                                format!("{pack}::{base}")
                            };
                            self.walk(&child, emit);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
