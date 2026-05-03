//! `#[xs_sub]` proc-macro implementation (v1).
//!
//! Turns a Rust function with a high-level signature like
//!
//! ```ignore
//! #[xs_sub]
//! fn is_even(n: IV) -> bool { n % 2 == 0 }
//! ```
//!
//! into an `extern "C"` XS-callable trampoline that:
//!
//!   * takes `(my_perl: *mut PerlInterpreter, cv: *mut CV)`
//!   * extracts each declared argument from the Perl stack via the
//!     appropriate `Sv*` reader
//!   * runs the user's body
//!   * pushes the return value back onto the stack as a mortal SV
//!
//! v1 type set:
//!
//! | Rust type | as arg          | as return |
//! |-----------|-----------------|-----------|
//! | `IV`      | `SvIV(my_perl, sv)` | `Perl_sv_setiv` |
//! | `bool`    | (not supported) | `Perl_sv_setiv(_, _, val as IV)` |
//!
//! Other types will be added incrementally; passing an unsupported type
//! produces a clear compile-time diagnostic at the proc-macro boundary.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, FnArg, Ident, ItemFn, Pat, PatType, ReturnType, Type, TypePath,
};

#[derive(Clone, Copy)]
enum ArgKind {
    Iv,
}

#[derive(Clone, Copy)]
enum RetKind {
    Iv,
    Bool,
    Unit,
}

struct ArgSpec {
    name: Ident,
    kind: ArgKind,
}

pub fn xs_sub(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);

    let mut arg_specs: Vec<ArgSpec> = Vec::new();
    for arg in &func.sig.inputs {
        match arg {
            FnArg::Receiver(r) => {
                return error(r, "`#[xs_sub]` does not support `self` receiver");
            }
            FnArg::Typed(PatType { pat, ty, .. }) => {
                let name = match pat.as_ref() {
                    Pat::Ident(p) => p.ident.clone(),
                    other => return error(other, "`#[xs_sub]` argument must be a plain identifier"),
                };
                let kind = match classify_arg_type(ty) {
                    Some(k) => k,
                    None => {
                        return error(
                            ty,
                            "`#[xs_sub]` v1 only supports `IV` argument type",
                        );
                    }
                };
                arg_specs.push(ArgSpec { name, kind });
            }
        }
    }

    let ret_kind = match &func.sig.output {
        ReturnType::Default => RetKind::Unit,
        ReturnType::Type(_, ty) => match classify_ret_type(ty) {
            Some(k) => k,
            None => {
                return error(
                    ty,
                    "`#[xs_sub]` v1 only supports `IV`, `bool`, or `()` return",
                );
            }
        },
    };

    let fn_name = &func.sig.ident;
    let ret_ty_for_user = match &func.sig.output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    // Hidden inner helper that holds the user's body verbatim. Renaming
    // (rather than rewriting) preserves the user's *exact* signature
    // — type names, attributes, generics — so things like
    // `use libperl_rs::IV` stay "used" and rustdoc / IDE tools see a
    // normal Rust function.
    let body_fn_name = quote::format_ident!("__xs_body_{}", fn_name);
    let mut body_fn_item = func.clone();
    body_fn_item.sig.ident = body_fn_name.clone();
    body_fn_item.vis = syn::Visibility::Inherited;
    let body_fn = quote! {
        #[allow(non_snake_case)]
        #[inline]
        #body_fn_item
    };
    let user_arg_call: Vec<TokenStream2> =
        arg_specs.iter().map(|s| { let n = &s.name; quote! { #n } }).collect();

    let arg_count = arg_specs.len();
    let usage_str = arg_specs
        .iter()
        .map(|a| a.name.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let usage_cstring = std::ffi::CString::new(usage_str)
        .expect("usage string contains interior nul");
    let usage_lit = syn::LitCStr::new(usage_cstring.as_c_str(), proc_macro2::Span::call_site());

    // Per-argument extraction
    let arg_extractions: Vec<TokenStream2> = arg_specs
        .iter()
        .enumerate()
        .map(|(i, spec)| {
            let name = &spec.name;
            let i_lit = syn::Index::from(i);
            match spec.kind {
                ArgKind::Iv => quote! {
                    let #name: ::libperl_rs::IV = unsafe {
                        let svp = ::libperl_rs::PL_stack_base!(my_perl).add(__ax + #i_lit);
                        ::libperl_rs::SvIV(my_perl, *svp)
                    };
                },
            }
        })
        .collect();

    // Return value push
    let return_push: TokenStream2 = match ret_kind {
        RetKind::Iv => quote! {
            let __targ = unsafe { ::libperl_rs::Perl_sv_newmortal(my_perl) };
            unsafe { ::libperl_rs::Perl_sv_setiv(my_perl, __targ, __ret); }
            unsafe { *__sp = __targ; }
            __set_sp_for_n(1);
        },
        RetKind::Bool => quote! {
            let __targ = unsafe { ::libperl_rs::Perl_sv_newmortal(my_perl) };
            unsafe {
                ::libperl_rs::Perl_sv_setiv(my_perl, __targ, __ret as ::libperl_rs::IV);
            }
            unsafe { *__sp = __targ; }
            __set_sp_for_n(1);
        },
        RetKind::Unit => quote! {
            let _ = __ret;
            __set_sp_for_n(0);
        },
    };

    let trampoline = quote! {
        #body_fn

        #[unsafe(no_mangle)]
        #[allow(unused_variables, unreachable_code)]
        pub extern "C" fn #fn_name(
            my_perl: *mut ::libperl_rs::PerlInterpreter,
            cv: *mut ::libperl_rs::CV,
        ) {
            if my_perl.is_null() {
                return;
            }

            // dXSARGS-equivalent: pop the mark, derive ax / items / sp.
            // Don't pin the offset type — `Stack_off_t` was introduced in
            // Perl 5.36; older Perls expose the same field as `SSize_t` /
            // `I32`. Letting rustc infer keeps the macro portable.
            let __sp: *mut *mut ::libperl_rs::SV = ::libperl_rs::PL_stack_sp!(my_perl);
            let __mark_ax = unsafe {
                *::libperl_rs::PL_markstack_ptr!(my_perl)
            };
            // POPMARK: decrement markstack_ptr
            unsafe {
                (*my_perl).Imarkstack_ptr = (*my_perl).Imarkstack_ptr.sub(1);
            }
            let __mark = unsafe {
                ::libperl_rs::PL_stack_base!(my_perl).add(__mark_ax as usize)
            };
            let __ax: usize = (__mark_ax as usize).wrapping_add(1);
            let __items = unsafe { __sp.offset_from(__mark) };

            // Arity check
            if __items != #arg_count as isize {
                unsafe {
                    ::libperl_rs::Perl_croak_xs_usage(cv, #usage_lit.as_ptr());
                }
                return;
            }

            // Argument extraction
            #( #arg_extractions )*

            // User body — invoked via the hidden helper fn so the user's
            // typed signature is preserved for tooling and import-usage
            // analysis.
            let __ret: #ret_ty_for_user = #body_fn_name( #( #user_arg_call ),* );

            // SP setter helper, centralised so the off-by-one is not at
            // each call site
            let __set_sp_for_n = |n: usize| unsafe {
                (*my_perl).Istack_sp =
                    ::libperl_rs::PL_stack_base!(my_perl).add(__ax + n - 1);
            };

            // Return value push
            #return_push
        }
    };

    trampoline.into()
}

fn classify_arg_type(ty: &Type) -> Option<ArgKind> {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let last = path.segments.last()?;
            match last.ident.to_string().as_str() {
                "IV" => Some(ArgKind::Iv),
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_ret_type(ty: &Type) -> Option<RetKind> {
    match ty {
        Type::Tuple(t) if t.elems.is_empty() => Some(RetKind::Unit),
        Type::Path(TypePath { path, .. }) => {
            let last = path.segments.last()?;
            match last.ident.to_string().as_str() {
                "IV" => Some(RetKind::Iv),
                "bool" => Some(RetKind::Bool),
                _ => None,
            }
        }
        _ => None,
    }
}

fn error<T: quote::ToTokens>(spanned: T, msg: &str) -> TokenStream {
    syn::Error::new_spanned(spanned, msg).to_compile_error().into()
}
