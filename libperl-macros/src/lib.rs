//! Procedural macros for libperl-rs.
//!
//! Currently exposes only a stub `#[thx]` attribute. The intended behavior is
//! documented in `docs/plan/README.md` §3.6 and §3.11; the actual code
//! transformation will be filled in during Step 3 (XS support).
//!
//! Threading mode is selected at proc-macro compile time via
//! `cfg(perl_useithreads)`, set by `build.rs`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

/// `#[thx]` — primitive THX-aware function attribute.
///
/// In threaded build, this should prepend `my_perl: *mut PerlInterpreter` to
/// the function's parameter list (see plan §3.6, §3.8). In non-threaded build,
/// it should leave the function unchanged.
///
/// **Status: stub.** This stub returns the input unchanged in both modes; the
/// real transformation is deferred to Step 1/3 of the plan.
#[proc_macro_attribute]
pub fn thx(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    // TODO: in threaded build, splice `my_perl: *mut PerlInterpreter` at the
    // head of `func.sig.inputs`. For now, return the function untouched so
    // that downstream crates can already attach `#[thx]` without breakage.
    let expanded = quote! { #func };
    expanded.into()
}
