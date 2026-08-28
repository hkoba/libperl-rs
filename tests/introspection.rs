//! End-to-end exercise of the Step 2 introspection layer: stash walk →
//! Cv → (Gv names / file / line, first COP, pad names, OP-tree walk in
//! both orders, SvKind classification after `run`).
//!
//! Single `#[test]` on purpose: each integration-test *file* is its own
//! process, but multiple `#[test]`s in one file share it, and one
//! process should host at most one interpreter at a time (on
//! non-threaded perl the interpreter state is global).

use libperl_rs::*;

// Note the `our @hello` on line 8: since the sub-ref-in-stash
// optimisation (perl 5.22+), a simple `sub name {...}` occupies its
// stash slot as a bare RV→CV with *no glob* — the walker reports
// those with `gv: None`. Adding a second slot (`@hello`) upgrades the
// entry to a real glob, so exactly the `hello` entry is guaranteed to
// carry a `Gv` for the location assertions below.
const SCRIPT: &str = "\
package Foo;
sub add { my ($x, $y) = @_; return $x + $y }
sub id { $_[0] }
package Foo::Bar;
sub nested { 42 }
package main;
sub hello { my ($who) = @_; \"hi $who\" }
our @hello;
$main::ref = \\&Foo::add;
";

struct Entry {
    package: String,
    name: String,
    cv: Cv,
    gv_file: Option<String>,
    gv_line: Option<u32>,
}

#[test]
fn introspection_walk() {
    let mut perl = Perl::new();
    let empty: &[&str] = &[];
    let rc = perl.parse(&["", "-e", SCRIPT], empty);
    assert_eq!(rc, 0, "perl_parse failed");

    // ─── stash walk: collect every named sub ────────────────────────
    let mut subs: Vec<Entry> = Vec::new();
    let mut walker = StashWalker::new(&perl);
    walker.walk("main", &mut |e| {
        subs.push(Entry {
            package: e.package.clone(),
            name: e.name.clone(),
            cv: e.cv,
            gv_file: e.gv.as_ref().and_then(|g| g.file()),
            gv_line: e.gv.as_ref().and_then(|g| g.line()),
        });
    });
    let find = |pkg: &str, name: &str| {
        subs.iter().find(|e| e.package == pkg && e.name == name)
    };

    let add = find("Foo", "add").expect("Foo::add not found by stash walk");
    let hello = find("main", "hello").expect("main::hello not found");
    assert!(
        find("Foo::Bar", "nested").is_some(),
        "nested package Foo::Bar not walked"
    );

    // ─── CV-level facts ─────────────────────────────────────────────
    assert_eq!(add.cv.file().as_deref(), Some("-e"));
    assert!(!add.cv.is_xsub());
    assert_eq!(
        add.cv.names(&perl).map(|(full, _)| full).as_deref(),
        Some("Foo::add")
    );

    // The first statement of Foo::add sits on line 2 of the -e script.
    let cop = add.cv.first_cop(&perl).expect("no COP found in Foo::add");
    assert_eq!(cop.line(), 2);
    assert_eq!(cop.file().as_deref(), Some("-e"));

    // GV location: hello's glob (vivified by `our @hello`, see the
    // SCRIPT note) reports the defining sub's position, line 7.
    assert_eq!(hello.gv_file.as_deref(), Some("-e"));
    assert_eq!(hello.gv_line, Some(7));

    // ─── pad names ──────────────────────────────────────────────────
    let lexicals: Vec<String> =
        add.cv.pad_names().flatten().filter_map(|pn| pn.pv()).collect();
    assert!(
        lexicals.contains(&"$x".to_string()) && lexicals.contains(&"$y".to_string()),
        "pad names {lexicals:?} should contain $x and $y"
    );

    // ─── OP tree, both orders ───────────────────────────────────────
    fn tree_names(op: Op, out: &mut Vec<String>) {
        out.push(op.name().unwrap_or("<custom>").to_string());
        for kid in op.kids() {
            tree_names(kid, out);
        }
    }
    let mut names = Vec::new();
    tree_names(add.cv.root_op().expect("no root op"), &mut names);
    assert!(
        names.iter().any(|n| n == "add"),
        "tree-order walk {names:?} should reach the add op"
    );

    let exec: Vec<String> = add
        .cv
        .start_op()
        .expect("no start op")
        .next_iter()
        .take(64) // straight-line sub; cap is belt-and-braces
        .map(|o| o.name().unwrap_or("<custom>").to_string())
        .collect();
    assert!(
        exec.iter().any(|n| n == "add"),
        "execution-order walk {exec:?} should reach the add op"
    );

    // ─── run(), then SvKind over a live value ───────────────────────
    assert_eq!(perl.run(), 0, "perl_run failed");
    let rv = perl.get_sv("ref", 0).expect("$main::ref not found after run");
    let SvKind::Ref(target) = rv.kind() else {
        panic!("$main::ref should be a reference, got {:?}", rv.kind());
    };
    let SvKind::Code(cv) = target.kind() else {
        panic!("referent should be code, got {:?}", target.kind());
    };
    assert_eq!(
        cv.as_ptr(),
        add.cv.as_ptr(),
        "\\&Foo::add should be the same CV the stash walk found"
    );
}
