//! Procedural macros for libperl-rs.
//!
//! - `#[thx]` — function attribute that splices `my_perl: *mut
//!   PerlInterpreter` as the first parameter in threaded builds and is a
//!   no-op in non-threaded builds. Lets a single Rust source compile
//!   against both `MULTIPLICITY` modes without manual `cfg` branches.
//! - `#[xs_sub]` — function attribute that turns a high-level Rust
//!   signature like `fn is_even(n: IV) -> bool { ... }` into a complete
//!   XS-callable `extern "C"` trampoline. (Phase 3.2 — TBD.)
//! - `xs_boot!` — declarative macro that emits the module's `boot_<name>`
//!   entry. (Phase 3.3 — TBD.)
//!
//! Threading mode is selected at proc-macro compile time via
//! `cfg(perl_useithreads)`, set by `build.rs`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn};

/// `#[thx]` — splice `my_perl: *mut ::libperl_sys::PerlInterpreter` as the
/// first parameter of `fn` in threaded builds; pass through unchanged in
/// non-threaded builds.
///
/// The injected name `my_perl` matches the C convention (`aTHX_`) and the
/// project's naming rule (see `docs/plan/README.md` §3.8). The
/// fully-qualified path `::libperl_sys::PerlInterpreter` is hard-coded
/// rather than `$crate::PerlInterpreter` because proc-macros emit raw
/// tokens — the path is resolved at the *call site* of `#[thx]`.
///
/// Examples
/// --------
///
/// Source:
///
/// ```ignore
/// #[thx]
/// fn helper(sv: *mut SV) -> i32 { /* ... */ }
/// ```
///
/// Threaded expansion:
///
/// ```ignore
/// fn helper(my_perl: *mut ::libperl_sys::PerlInterpreter, sv: *mut SV) -> i32 { /* ... */ }
/// ```
///
/// Non-threaded expansion: identical to the input.
#[proc_macro_attribute]
pub fn thx(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    if cfg!(perl_useithreads) {
        let my_perl_param: FnArg = syn::parse_quote! {
            my_perl: *mut ::libperl_sys::PerlInterpreter
        };
        func.sig.inputs.insert(0, my_perl_param);
    }
    quote!(#func).into()
}
