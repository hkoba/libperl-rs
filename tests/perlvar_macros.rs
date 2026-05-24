//! End-to-end smoke test for PERLVAR-driven `PL_xxx!()` macros emitted by
//! libperl-macrogen.
//!
//! The macros live in libperl-sys (where `macro_bindings.rs` is included).
//! Calling them from a *downstream* crate (libperl-rs) bypasses the
//! intra-crate `macro_expanded_macro_exports_accessed_by_absolute_paths`
//! lint, so this is the natural place to exercise expansion.

use libperl_sys::{perl_alloc, perl_construct, perl_destruct, OP, PL_main_start};

/// `PL_main_start!(my_perl)` should compile, type-check, and (with a
/// freshly-constructed but un-parsed interpreter) return the null `OP*`.
#[test]
fn pl_main_start_macro() {
    let my_perl = unsafe { perl_alloc() };
    assert!(!my_perl.is_null(), "perl_alloc returned null");
    unsafe { perl_construct(my_perl) };

    let start: *mut OP = PL_main_start!(my_perl);
    // Before perl_parse, PL_main_start is null. The test value is the
    // type and the absence of UB, not the runtime content.
    assert!(start.is_null(), "PL_main_start is non-null before perl_parse");

    unsafe { perl_destruct(my_perl) };
}
