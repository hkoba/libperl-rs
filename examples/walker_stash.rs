//! North-star example #2a for Step 2 (`docs/plan/README.md` §4
//! Step 2.3): enumerate every named sub in the package tree via
//! [`StashWalker`], then dump the OP tree (tree order, with statement
//! line numbers) of the subs defined in the main script file.
//!
//! ```text
//! cargo run --example walker_stash -- -e 'sub hello { my ($who) = @_; print "hi $who\n" }'
//! ```
//!
//! Port of libperl-proto0's `105_scan_stash.rs` /
//! `106_scan_allpackage.rs` onto the new newtype layer.

use std::env;

use libperl_rs::{Op, Perl, StashWalker};

fn print_tree(perl: &Perl, op: Op, level: usize) {
    let name = op.name().unwrap_or("<custom>");
    let loc = op
        .as_cop(perl)
        .map(|cop| format!(" (line {})", cop.line()))
        .unwrap_or_default();
    println!("{}{name}{loc}", "  ".repeat(level));
    for kid in op.kids() {
        print_tree(perl, kid, level + 1);
    }
}

fn main() {
    let mut perl = Perl::new();
    perl.parse_env_args(env::args(), env::vars());

    let sv0 = perl.get_sv("0", 0).expect("$0 is always set");
    let main_file = String::from_utf8_lossy(sv0.pv(&perl)).into_owned();
    println!("$0 = {main_file:?}");

    let mut walker = StashWalker::new(&perl);
    walker.walk("main", &mut |e| {
        let file = e.cv.file();
        println!("sub {}::{} file {:?}", e.package, e.name, file);
        if file.as_deref() == Some(main_file.as_str()) {
            if let Some(root) = e.cv.root_op() {
                print_tree(&perl, root, 1);
            }
        }
    });
}
