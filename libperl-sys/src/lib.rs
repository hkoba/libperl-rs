//! # libperl-sys
//!
#![doc = concat!(
    "**Built against Perl ", env!("LIBPERL_SYS_PERL_VERSION"),
    " (", env!("LIBPERL_SYS_PERL_THREADED"),
    ", `", env!("LIBPERL_SYS_PERL_ARCHNAME"), "`).**",
)]
//!
//! The function signatures, `PL_*` globals, and `Sv*` / `Av*` / `Hv*`
//! helpers documented below reflect this specific Perl. Different
//! Perl versions may have minor signature differences (added /
//! removed functions, changed integer widths, threading-mode
//! variations). Use the [`PERL_VERSION`] / [`PERL_THREADED`] /
//! [`PERL_ARCHNAME`] constants for runtime identification.
//!
//! Low-level, raw FFI declarations for the Perl 5 C API (`libperl`).
//! Generated at build time by `bindgen` (regular C declarations) plus
//! [`libperl-macrogen`](https://docs.rs/libperl-macrogen) (the C
//! macros and `static inline` functions that `bindgen` skips).
//!
//! This crate is the unsafe foundation under
//! [`libperl-rs`](https://docs.rs/libperl-rs); most users want that
//! safer wrapper. Reach for `libperl-sys` directly when you need an
//! API element that hasn't been wrapped yet, or when you're writing
//! a sibling crate at the same layer.
//!
//! ## What you get
//!
//! Re-exported at the crate root:
//!
//! - `Perl_*` extern functions and `PL_*` mutable statics (from
//!   bindgen),
//! - `Sv*` / `Av*` / `Hv*` / `PL_xxx!()` macro helpers and inline
//!   wrappers (from libperl-macrogen) — these unify the threaded vs
//!   non-threaded calling conventions so the same source builds
//!   against both `MULTIPLICITY` modes,
//! - opcode → name lookup table ([`conv_opcode`]) and per-function
//!   signature dictionary ([`sigdb`]) for downstream codegen.
//!
//! ## Safety
//!
//! Every public item here is `unsafe` to use. Even reading a `PL_*`
//! global requires the right interpreter context, and Perl's API
//! uses raw `*mut` pointers ubiquitously.
//!
//! ## Build requirements
//!
//! - A working Perl 5 install with development headers
//!   (`Perl.h`, `EXTERN.h`, ...). Typical packages: `perl-dev`,
//!   `perl-devel`.
//! - LLVM / libclang (for `bindgen`).
//! - Internet access at first build (libperl-macrogen downloads a
//!   pre-extracted apidoc snapshot from GitHub Releases).
//!
//! Threaded vs non-threaded Perl is auto-detected — no feature flag
//! to set.

pub mod perl_core;
pub use perl_core::*;

pub mod conv_opcode;

pub mod sigdb;

/// Perl version this binding was generated against (e.g. `"5.38.4"`).
pub const PERL_VERSION:  &str = env!("LIBPERL_SYS_PERL_VERSION");

/// `"threaded"` if the target Perl was built with `useithreads`,
/// `"non-threaded"` otherwise. Threading mode determines whether
/// most Perl C API functions take a leading `my_perl: *mut PerlInterpreter`
/// parameter.
pub const PERL_THREADED: &str = env!("LIBPERL_SYS_PERL_THREADED");

/// Perl `archname` (e.g. `"x86_64-linux-thread-multi"` or
/// `"x86_64-linux-gnu"`). Mostly informational; the more useful
/// invariants are in [`PERL_VERSION`] and [`PERL_THREADED`].
pub const PERL_ARCHNAME: &str = env!("LIBPERL_SYS_PERL_ARCHNAME");

use std::ffi::CStr;

// use std::os::raw::{c_char, c_int /*, c_void, c_schar*/};

fn core_op_name(o: &op) -> Option<String> {
    let ty = o.op_type();
    if (ty as usize) < unsafe {PL_op_name.len()} {
        let op_name = unsafe {CStr::from_ptr(PL_op_name[ty as usize])};
        Some(String::from(op_name.to_str().unwrap()))
    } else {
        None
    }
}

impl std::fmt::Display for op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ {:?}={:#?} {:?} }}"
               , core_op_name(&self)
               , (self as *const op)
               , self)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let perl = unsafe { super::perl_alloc() };
        unsafe {
            super::perl_construct(perl);
        };
    }

    // Note: a smoke test for the PERLVAR-driven `PL_xxx!($my_perl)` macros
    // would naturally live here, but `#[macro_export]` macros emitted via
    // `include!()` are unreachable by absolute path within the *defining*
    // crate (rejected by the
    // `macro_expanded_macro_exports_accessed_by_absolute_paths` lint, which
    // is on by default and slated to become a hard error). The smoke test
    // is in `libperl-rs/tests/perlvar_macros.rs` instead, where cross-crate
    // access goes through the normal path resolver and is unaffected.

    #[test]
    fn sigdb_lookup() {
        use super::sigdb::{FN_BY_NAME, FUNCS};

        // Test that FN_BY_NAME lookup works
        if let Some(id) = FN_BY_NAME.get("Perl_sv_isbool") {
            let sig = &FUNCS[id.0 as usize];
            assert_eq!(sig.name, "Perl_sv_isbool");
            assert!(!sig.ret.is_empty());
        }

        // Test perl_alloc
        let id = FN_BY_NAME.get("perl_alloc").expect("perl_alloc should exist");
        let sig = &FUNCS[id.0 as usize];
        assert_eq!(sig.name, "perl_alloc");
        assert!(sig.ret.contains("PerlInterpreter"));
    }
}
