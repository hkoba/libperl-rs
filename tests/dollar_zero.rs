//! `$0 = ...` writes the new process title over the memory that
//! `perl_parse` was given as `argv` (`S_set_dollarzero`), so that memory
//! has to outlive `parse` and be laid out the way perl sizes it.
//!
//! Single `#[test]`: one interpreter per process.

use std::ffi::CStr;

use libperl_rs::*;
use libperl_sys::{PL_origalen, PL_origargv};

/// Longer than the whole argument block, so the title gets truncated.
const TITLE_LEN: usize = 300;

#[test]
fn assigning_dollar_zero_writes_into_the_owned_argv() {
    let script = format!("$0 = 'x' x {TITLE_LEN}; our $seen = $0;");
    let title = "x".repeat(TITLE_LEN);
    let args = ["libperl-rs-test", "-e", script.as_str()];
    let mut perl = Perl::new();
    let rc = perl.parse(&args, &["LIBPERL_RS_DOLLAR_ZERO=1"]);
    assert_eq!(rc, 0, "perl_parse failed");

    // perl measured the whole argument block as $0 scratch space,
    // exactly as for a real process's contiguous argv.
    let block_len: usize = args.iter().map(|a| a.len() + 1).sum();
    let origalen = PL_origalen!(perl.as_ptr()) as usize;
    assert_eq!(origalen, block_len, "PL_origalen covers all arguments");
    assert!(origalen < TITLE_LEN);

    // Spray the heap a little so that a dangling PL_origargv would be
    // overwritten before the assignment runs.
    let junk: Vec<Vec<u8>> = (0..64).map(|i| vec![0xA5; 8 + i]).collect();
    drop(junk);

    assert_eq!(perl.run(), 0, "perl_run failed");

    let seen = perl.get_sv("main::seen", 0).expect("$main::seen");
    assert_eq!(seen.pv(&perl), title.as_bytes(), "$0 reads back in full");

    // The title landed in our argument block, truncated to its length,
    // and the remaining argv slots were cleared.
    let argv = PL_origargv!(perl.as_ptr());
    let argv0 = unsafe { CStr::from_ptr(*argv) };
    assert_eq!(argv0.to_bytes(), &title.as_bytes()[..origalen - 1]);
    assert!(unsafe { *argv.add(1) }.is_null(), "PL_origargv[1] cleared by set_dollarzero");
}
