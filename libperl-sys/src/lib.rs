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
//! - `PL_xxx_ptr!()` pointer accessors (read *and write* the
//!   interpreter variables through one primitive) and the [`thx`]
//!   calling-convention shim module — together they let downstream
//!   crates touch raw interpreter state without `cfg`-forking on the
//!   threading mode (GH-20),
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

/// Threaded-style calling-convention shims (GH-20).
///
/// Every function here takes `my_perl: *mut PerlInterpreter` first.
/// The argument is forwarded when the wrapped function wants a context
/// and silently dropped when it does not — which covers both
/// non-threaded builds (no function takes a context) and the handful of
/// context-free functions on threaded builds. Downstream code can
/// therefore call `sys::thx::Perl_foo(my_perl, ...)` uniformly and
/// compile against both `MULTIPLICITY` modes unchanged.
///
/// Wraps both the bindgen externs (bindings.rs) and the
/// libperl-macrogen-generated inline functions (macro_bindings.rs) —
/// the latter also change signature with the threading mode. C variadic
/// functions (e.g. `Perl_croak`) cannot be wrapped in stable Rust and
/// are omitted; call them through the crate root with an explicit
/// `#[cfg(perl_useithreads)]` branch if you need them.
#[allow(
    non_snake_case,
    unused_imports,
    unused_unsafe,
    clippy::missing_safety_doc,
    clippy::too_many_arguments
)]
pub mod thx {
    include!(concat!(env!("OUT_DIR"), "/thx_bindings.rs"));

    // perl_core.rs と同じ <5.32 互換 (5.31 で S_SvREFCNT_dec →
    // Perl_SvREFCNT_dec 改名): shim は S_SvREFCNT_dec としてしか生成
    // されないので、thx 名前空間にも Perl_ 名の alias を張る。shim が
    // 既に呼び出し規約を正規化済みのため alias だけで足りる。
    // 5.20〜5.26 でも S_SvREFCNT_dec が生成されることは macrogen 0.1.12
    // (apidoc data 1.15) の multi-perl 成果物で確認済み。
    #[cfg(not(perlapi_ver32))]
    pub use self::S_SvREFCNT_dec as Perl_SvREFCNT_dec;

    // <5.26 互換: newAV / newHV / hv_store の関数形 (`Perl_` 名の
    // extern) は perl 5.26 で生まれた。5.24 以前はマクロのみだが、
    // macrogen がそのマクロを同シグネチャの inline fn として生成する
    // (5.20〜5.24 x 両モードの multi-perl 成果物で確認済み) ので、
    // その thx shim を `Perl_` 名でも使えるようにする。
    #[cfg(not(perlapi_ver26))]
    pub use self::newAV as Perl_newAV;
    #[cfg(not(perlapi_ver26))]
    pub use self::newHV as Perl_newHV;
    #[cfg(not(perlapi_ver26))]
    pub use self::hv_store as Perl_hv_store;
    // `Perl_sv_2iv` の extern も 5.26 生まれ (それ以前は sv_2iv_flags
    // のみ)。マクロ生成体 sv_2iv (= sv_2iv_flags(sv, SV_GMAGIC)) の
    // shim を同名で使えるようにする。
    #[cfg(not(perlapi_ver26))]
    pub use self::sv_2iv as Perl_sv_2iv;
}

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
