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

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{bracketed, parse_macro_input, Ident, LitStr, Token};

struct XsBootInput {
    package: LitStr,
    subs: Vec<Ident>,
}

mod kw {
    syn::custom_keyword!(package);
    syn::custom_keyword!(subs);
}

impl Parse for XsBootInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut package: Option<LitStr> = None;
        let mut subs: Option<Vec<Ident>> = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::package) {
                input.parse::<kw::package>()?;
                input.parse::<Token![=]>()?;
                package = Some(input.parse()?);
                input.parse::<Token![;]>()?;
            } else if lookahead.peek(kw::subs) {
                input.parse::<kw::subs>()?;
                input.parse::<Token![=]>()?;
                let content;
                bracketed!(content in input);
                let parsed: syn::punctuated::Punctuated<Ident, Token![,]> =
                    content.parse_terminated(Ident::parse, Token![,])?;
                subs = Some(parsed.into_iter().collect());
                input.parse::<Token![;]>()?;
            } else {
                return Err(lookahead.error());
            }
        }

        let package = package
            .ok_or_else(|| syn::Error::new(input.span(), "missing `package = \"...\";`"))?;
        let subs = subs
            .ok_or_else(|| syn::Error::new(input.span(), "missing `subs = [...];`"))?;
        Ok(Self { package, subs })
    }
}

pub fn xs_boot(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as XsBootInput);

    let pkg = parsed.package.value();
    let boot_ident_str = format!("boot_{}", pkg.replace("::", "__"));
    let boot_ident = Ident::new(&boot_ident_str, parsed.package.span());

    let n_subs = parsed.subs.len() as isize;

    let registrations = parsed.subs.iter().map(|sub| {
        let perl_name = format!("{pkg}::{sub}");
        let perl_name_cstring =
            std::ffi::CString::new(perl_name).expect("interior nul in sub name");
        let perl_name_lit =
            syn::LitCStr::new(perl_name_cstring.as_c_str(), sub.span());
        quote! {
            unsafe {
                ::libperl_rs::Perl_newXS_deffile(
                    my_perl,
                    #perl_name_lit.as_ptr(),
                    ::core::option::Option::Some(#sub),
                );
            }
        }
    });

    let expanded = quote! {
        #[unsafe(no_mangle)]
        pub extern "C" fn #boot_ident(
            my_perl: *mut ::libperl_rs::PerlInterpreter,
            _cv: *mut ::libperl_rs::CV,
        ) {
            if my_perl.is_null() { return; }
            #( #registrations )*
            unsafe {
                ::libperl_rs::Perl_xs_boot_epilog(my_perl, #n_subs);
            }
        }
    };
    expanded.into()
}
