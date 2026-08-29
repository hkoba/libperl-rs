//! `xs_boot!` declarative macro implementation.
//!
//! Syntax:
//!
//! ```ignore
//! xs_boot! {
//!     package = "Mytest";
//!     subs = [is_even, add];
//! }
//! ```
//!
//! Expands to a single `extern "C" fn boot_<modname>(my_perl, _cv)` that
//! registers each listed sub with `Perl_newXS_deffile` and finishes with
//! `Perl_xs_boot_epilog(my_perl, n_subs)`.
//!
//! The `<modname>` portion of the boot function name is derived from the
//! package literal by replacing `::` with `__` (Perl XS convention) and
//! prepending `boot_`. Example: `Foo::Bar` → `boot_Foo__Bar`.
//!
//! Multiple packages
//! -----------------
//!
//! XSLoader calls exactly one boot function per shared library, but one
//! library may serve several Perl namespaces. Repeat the `package` /
//! `subs` pair to register into more than one, and name the boot function
//! explicitly with `module`:
//!
//! ```ignore
//! xs_boot! {
//!     module  = "LibPerlRs::PartialEval";   // -> boot_LibPerlRs__PartialEval
//!     package = "LibPerlRs::PartialEval";
//!     subs    = [engine_version, _compile];
//!     package = "LibPerlRs::EvalCapture";
//!     subs    = [_start, _stop];
//! }
//! ```
//!
//! This mirrors the `MODULE = ...  PACKAGE = ...` convention of hand-written
//! `.xs` files: `module` picks the boot symbol, `package` switches the
//! namespace subsequent `subs` are registered into. `module` is optional and
//! defaults to the first `package`, so the single-package form above stays
//! valid and unchanged.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{bracketed, parse_macro_input, Ident, LitStr, Token};

struct XsBootInput {
    /// Explicit boot-function name; falls back to the first group's package.
    module: Option<LitStr>,
    /// One `(package, subs)` pair per `package = "..."; subs = [...];` block.
    groups: Vec<(LitStr, Vec<Ident>)>,
}

mod kw {
    syn::custom_keyword!(module);
    syn::custom_keyword!(package);
    syn::custom_keyword!(subs);
}

impl Parse for XsBootInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut module: Option<LitStr> = None;
        let mut groups: Vec<(LitStr, Vec<Ident>)> = Vec::new();

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::module) {
                input.parse::<kw::module>()?;
                input.parse::<Token![=]>()?;
                let lit: LitStr = input.parse()?;
                input.parse::<Token![;]>()?;
                if module.is_some() {
                    return Err(syn::Error::new(
                        lit.span(),
                        "duplicate `module = \"...\";` (only one boot function \
                         can be emitted per macro invocation)",
                    ));
                }
                module = Some(lit);
            } else if lookahead.peek(kw::package) {
                input.parse::<kw::package>()?;
                input.parse::<Token![=]>()?;
                let pkg: LitStr = input.parse()?;
                input.parse::<Token![;]>()?;
                // Each `package` opens a group that `subs` must close, so
                // that every sub has an unambiguous namespace.
                if !input.peek(kw::subs) {
                    return Err(syn::Error::new(
                        pkg.span(),
                        "`package = \"...\";` must be followed by `subs = [...];`",
                    ));
                }
                input.parse::<kw::subs>()?;
                input.parse::<Token![=]>()?;
                let content;
                bracketed!(content in input);
                let parsed: syn::punctuated::Punctuated<Ident, Token![,]> =
                    content.parse_terminated(Ident::parse, Token![,])?;
                input.parse::<Token![;]>()?;
                groups.push((pkg, parsed.into_iter().collect()));
            } else if lookahead.peek(kw::subs) {
                let tok = input.parse::<kw::subs>()?;
                return Err(syn::Error::new(
                    tok.span,
                    "`subs = [...];` without a preceding `package = \"...\";`",
                ));
            } else {
                return Err(lookahead.error());
            }
        }

        if groups.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "missing `package = \"...\"; subs = [...];`",
            ));
        }
        Ok(Self { module, groups })
    }
}

impl XsBootInput {
    /// The `LitStr` the boot symbol is derived from: `module` when given,
    /// otherwise the first `package`. Keeping this in one place is what
    /// makes the single-package form byte-for-byte compatible.
    fn boot_pkg(&self) -> &LitStr {
        self.module.as_ref().unwrap_or(&self.groups[0].0)
    }

    /// `Foo::Bar` -> `boot_Foo__Bar` (Perl XS convention).
    fn boot_symbol(&self) -> String {
        format!("boot_{}", self.boot_pkg().value().replace("::", "__"))
    }

    /// Fully qualified Perl names, in registration order.
    fn perl_names(&self) -> Vec<String> {
        self.groups
            .iter()
            .flat_map(|(pkg, subs)| {
                let pkg = pkg.value();
                subs.iter().map(move |sub| format!("{pkg}::{sub}"))
            })
            .collect()
    }
}

pub fn xs_boot(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as XsBootInput);

    // The boot symbol XSLoader looks up. `module` when given, otherwise the
    // first package — which keeps the single-package form working verbatim.
    let boot_ident = Ident::new(&parsed.boot_symbol(), parsed.boot_pkg().span());

    // `Perl_xs_boot_epilog`'s `ax` parameter is `isize` in modern Perl
    // (5.40+) but was `I32` (= `i32`) in older Perls. Emit a usize
    // literal and `as _` so rustc infers the right integer type.
    let n_subs: usize = parsed.groups.iter().map(|(_, subs)| subs.len()).sum();

    // libperl-macros' `build.rs` sets `cfg(perl_useithreads)` at proc-
    // macro compile time. In threaded build the boot fn takes my_perl
    // and forwards it to every Perl_* call; in non-threaded build the
    // FFI signatures don't have a my_perl parameter at all.
    let threaded = cfg!(perl_useithreads);

    let boot_params = if threaded {
        quote! {
            my_perl: *mut ::libperl_rs::PerlInterpreter,
            _cv: *mut ::libperl_rs::CV,
        }
    } else {
        quote! { _cv: *mut ::libperl_rs::CV, }
    };

    let null_check = if threaded {
        quote! { if my_perl.is_null() { return; } }
    } else {
        quote! {}
    };

    // Qualified Perl names and the Rust idents they dispatch to, both in
    // registration order (groups flattened). `perl_names` is the single
    // source of truth the unit tests below check.
    let perl_names = parsed.perl_names();
    let sub_idents: Vec<&Ident> = parsed
        .groups
        .iter()
        .flat_map(|(_, subs)| subs.iter())
        .collect();

    let mut registrations: Vec<TokenStream2> = Vec::with_capacity(n_subs);
    for (perl_name, sub) in perl_names.iter().zip(sub_idents) {
        let perl_name_cstring =
            std::ffi::CString::new(perl_name.as_str()).expect("interior nul in sub name");
        let perl_name_lit = syn::LitCStr::new(perl_name_cstring.as_c_str(), sub.span());
        registrations.push(if threaded {
            quote! {
                unsafe {
                    ::libperl_rs::Perl_newXS_deffile(
                        my_perl,
                        #perl_name_lit.as_ptr(),
                        ::core::option::Option::Some(#sub),
                    );
                }
            }
        } else {
            quote! {
                unsafe {
                    ::libperl_rs::Perl_newXS_deffile(
                        #perl_name_lit.as_ptr(),
                        ::core::option::Option::Some(#sub),
                    );
                }
            }
        });
    }

    let epilog_call = if threaded {
        quote! {
            unsafe {
                ::libperl_rs::Perl_xs_boot_epilog(my_perl, #n_subs as _);
            }
        }
    } else {
        quote! {
            unsafe {
                ::libperl_rs::Perl_xs_boot_epilog(#n_subs as _);
            }
        }
    };

    let expanded = quote! {
        #[unsafe(no_mangle)]
        pub extern "C" fn #boot_ident( #boot_params ) {
            #null_check
            #( #registrations )*
            #epilog_call
        }
    };
    expanded.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> XsBootInput {
        syn::parse_str(src).expect("should parse")
    }

    fn err(src: &str) -> String {
        syn::parse_str::<XsBootInput>(src)
            .err()
            .expect("should fail to parse")
            .to_string()
    }

    // --- backward compatibility: the single-package form is unchanged ---

    #[test]
    fn single_package_boot_symbol_and_names() {
        let input = parse(r#"package = "Mytest"; subs = [is_even, round];"#);
        assert_eq!(input.boot_symbol(), "boot_Mytest");
        assert_eq!(input.perl_names(), ["Mytest::is_even", "Mytest::round"]);
    }

    #[test]
    fn nested_package_uses_double_underscore() {
        let input = parse(r#"package = "Foo::Bar"; subs = [baz];"#);
        assert_eq!(input.boot_symbol(), "boot_Foo__Bar");
        assert_eq!(input.perl_names(), ["Foo::Bar::baz"]);
    }

    // --- multi-package ---

    #[test]
    fn module_names_the_boot_symbol() {
        let input = parse(
            r#"module = "LibPerlRs::PartialEval";
               package = "LibPerlRs::PartialEval"; subs = [_compile];
               package = "LibPerlRs::EvalCapture"; subs = [_start, _stop];"#,
        );
        assert_eq!(input.boot_symbol(), "boot_LibPerlRs__PartialEval");
        assert_eq!(
            input.perl_names(),
            [
                "LibPerlRs::PartialEval::_compile",
                "LibPerlRs::EvalCapture::_start",
                "LibPerlRs::EvalCapture::_stop",
            ]
        );
    }

    #[test]
    fn omitted_module_falls_back_to_the_first_package() {
        let input = parse(
            r#"package = "Mytest2"; subs = [foo];
               package = "Mytest2::Extra"; subs = [bar];"#,
        );
        assert_eq!(input.boot_symbol(), "boot_Mytest2");
        assert_eq!(input.perl_names(), ["Mytest2::foo", "Mytest2::Extra::bar"]);
    }

    #[test]
    fn module_may_differ_from_every_package() {
        // XSLoader::load('Foo') booting subs that live only in Foo::Impl.
        let input = parse(r#"module = "Foo"; package = "Foo::Impl"; subs = [go];"#);
        assert_eq!(input.boot_symbol(), "boot_Foo");
        assert_eq!(input.perl_names(), ["Foo::Impl::go"]);
    }

    #[test]
    fn a_package_may_repeat() {
        // Not an error: registration order is simply the written order.
        let input = parse(
            r#"package = "A"; subs = [one];
               package = "B"; subs = [two];
               package = "A"; subs = [three];"#,
        );
        assert_eq!(input.perl_names(), ["A::one", "B::two", "A::three"]);
    }

    #[test]
    fn empty_subs_list_is_allowed() {
        let input = parse(r#"package = "A"; subs = [];"#);
        assert_eq!(input.boot_symbol(), "boot_A");
        assert!(input.perl_names().is_empty());
    }

    // --- diagnostics ---

    #[test]
    fn package_without_subs_is_rejected() {
        assert!(
            err(r#"package = "A";"#).contains("must be followed by `subs = [...];`"),
            "got: {}",
            err(r#"package = "A";"#)
        );
        let msg = err(r#"package = "A"; package = "B"; subs = [x];"#);
        assert!(
            msg.contains("must be followed by `subs = [...];`"),
            "got: {msg}"
        );
    }

    #[test]
    fn subs_without_package_is_rejected() {
        let msg = err(r#"subs = [x];"#);
        assert!(msg.contains("without a preceding `package"), "got: {msg}");
    }

    #[test]
    fn no_group_at_all_is_rejected() {
        let msg = err(r#"module = "A";"#);
        assert!(msg.contains("missing `package"), "got: {msg}");
    }

    #[test]
    fn duplicate_module_is_rejected() {
        let msg = err(r#"module = "A"; module = "B"; package = "A"; subs = [x];"#);
        assert!(msg.contains("duplicate `module"), "got: {msg}");
    }
}
