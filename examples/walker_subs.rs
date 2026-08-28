//! North-star example #2b for Step 2 (`docs/plan/README.md` §4
//! Step 2.3): the subs defined in the main script file, with their
//! location (first COP), lexicals (pad names, `my Foo $x` types
//! included) and the head of their execution-order op chain.
//!
//! ```text
//! cargo run --example walker_subs -- -e 'sub add { my ($x, $y) = @_; $x + $y }'
//! ```
//!
//! Port of libperl-proto0's `107_scan_subs_in_a_file.rs` /
//! `108`/`109` — the `my (...) = @_;` pattern-matching part of 109 is
//! left to a future analyzer layer; this shows the raw material such
//! an analyzer consumes.

use std::env;

use libperl_rs::{Perl, StashWalker};

fn main() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());

    let sv0 = perl.get_sv("0", 0).expect("$0 is always set");
    let main_file = String::from_utf8_lossy(sv0.pv(&perl)).into_owned();
    println!("$0 = {main_file:?}");

    let mut walker = StashWalker::new(&perl);
    walker.walk("main", &mut |e| {
        if e.cv.file().as_deref() != Some(main_file.as_str()) {
            return;
        }
        let qual = e
            .cv
            .names(&perl)
            .map(|(full, _)| full)
            .unwrap_or_else(|| format!("{}::{}", e.package, e.name));
        let line = e.cv.first_cop(&perl).map(|c| c.line());
        println!("sub {qual} (first statement at line {line:?})");

        let lexicals: Vec<String> = e
            .cv
            .pad_names()
            .flatten() // skip unnamed slots
            .filter_map(|pn| {
                // Target/temporary slots have a non-null but empty PV;
                // only real `my`/`our` names are interesting here.
                let pv = pn.pv().filter(|s| !s.is_empty())?;
                Some(match pn.type_stash_name() {
                    Some(t) => format!("{pv}: {t}"),
                    None => pv,
                })
            })
            .collect();
        println!("  lexicals: {lexicals:?}");

        let ops: Vec<&str> = e
            .cv
            .start_op()
            .into_iter()
            .flat_map(|s| s.next_iter())
            .take(20)
            .map(|o| o.name().unwrap_or("<custom>"))
            .collect();
        println!("  ops (execution order, first 20): {}", ops.join(" "));
    });
}
