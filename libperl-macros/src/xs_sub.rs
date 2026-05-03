//! `#[xs_sub]` proc-macro implementation.
//!
//! Turns a Rust function with a high-level signature into a complete
//! `extern "C"` XS-callable trampoline. See `docs/plan/README.md` §3.11
//! for the design rationale.
//!
//! Supported types (Phase 3.7 — perlxstut EXAMPLE 3):
//!
//! | Rust type     | as arg                        | as return                  |
//! |---------------|-------------------------------|----------------------------|
//! | `IV`          | `SvIV`                        | `Perl_sv_setiv`            |
//! | `UV`          | `SvUV`                        | `Perl_sv_setuv`            |
//! | `NV`          | `SvNV`                        | `Perl_sv_setnv`            |
//! | `bool`        | (not supported as arg)        | `Perl_sv_setiv(_ as IV)`   |
//! | `()`          | n/a                           | no return push             |
//! | `&mut IV`     | out-param: read + write back  | n/a (use `()` return)      |
//! | `&mut UV`     | out-param: read + write back  | n/a                        |
//! | `&mut NV`     | out-param: read + write back  | n/a                        |
//!
//! Other types produce a `compile_error!` pointing at the offending span.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, FnArg, Ident, ItemFn, Pat, PatType, ReturnType, Type, TypePath,
    TypeReference,
};

/// Scalar SV flavor — IV / UV / NV. Common to in-args, out-args, and
/// return values; each maps to a specific `Sv*` reader and `Perl_sv_set*`
/// writer.
#[derive(Clone, Copy)]
enum SvFlavor {
    Iv,
    Uv,
    Nv,
}

#[derive(Clone, Copy)]
enum ArgKind {
    /// Read-only scalar arg, e.g. `n: IV` or `x: NV`.
    In(SvFlavor),
    /// Out-parameter, e.g. `arg: &mut NV`. Read at entry, written back
    /// to the caller's SV after the body returns.
    Out(SvFlavor),
    /// `&CStr` — NUL-terminated byte string borrowed from the SV's PV
    /// buffer. No UTF-8 validation.
    InCStr,
    /// `&str` — same as InCStr but UTF-8 validated. On invalid UTF-8
    /// the trampoline croaks.
    InStr,
}

#[derive(Clone, Copy)]
enum RetKind {
    Scalar(SvFlavor),
    Bool,
    Unit,
    /// `String` — the bytes are pushed via `Perl_sv_setpvn`.
    String_,
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
                            "`#[xs_sub]` argument must be `IV` / `UV` / `NV` \
                             or `&mut IV` / `&mut UV` / `&mut NV`",
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
                    "`#[xs_sub]` return must be `()` / `bool` / `IV` / `UV` / `NV`",
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
    // Build the body call's argument list — out-params get `&mut`,
    // string args are passed as the borrowed slice we built locally.
    let user_arg_call: Vec<TokenStream2> = arg_specs
        .iter()
        .map(|s| {
            let n = &s.name;
            match s.kind {
                ArgKind::In(_) | ArgKind::InCStr | ArgKind::InStr => quote! { #n },
                ArgKind::Out(_) => quote! { &mut #n },
            }
        })
        .collect();

    let arg_count = arg_specs.len();
    let usage_str = arg_specs
        .iter()
        .map(|a| a.name.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let usage_cstring = std::ffi::CString::new(usage_str)
        .expect("usage string contains interior nul");
    let usage_lit = syn::LitCStr::new(usage_cstring.as_c_str(), proc_macro2::Span::call_site());

    // ─── Threading-mode-dependent fragments ────────────────────────
    // libperl-macros' `build.rs` sets `cfg(perl_useithreads)` at proc-
    // macro compile time, so we can pick different code-gen paths here.
    // - threaded:     `Perl_*(my_perl, ...)`, `(*my_perl).I<field>`
    // - non-threaded: `Perl_*(...)`,          `$crate::PL_<global>`
    let threaded = cfg!(perl_useithreads);

    // `my_perl,` (with comma) for use as an FFI call's first arg in
    // threaded build, empty in non-threaded build.
    let myperl_arg_prefix: TokenStream2 = if threaded { quote! { my_perl, } } else { quote! {} };
    // `my_perl` (without comma) for FFI calls that take *only* my_perl
    // in threaded mode.
    let myperl_arg_only: TokenStream2 = if threaded { quote! { my_perl } } else { quote! {} };

    // (a) trampoline parameter list
    let trampoline_params = if threaded {
        quote! {
            my_perl: *mut ::libperl_rs::PerlInterpreter,
            cv: *mut ::libperl_rs::CV,
        }
    } else {
        quote! { cv: *mut ::libperl_rs::CV, }
    };

    // (b) my_perl null-check (only relevant in threaded build).
    // In non-threaded build, `my_perl` is not a trampoline parameter, so
    // we synthesise a null stub so that `PL_xxx!(my_perl)` macros inside
    // the trampoline body still parse — they discard the stub and use
    // the global `PL_xxx` static directly.
    let null_check = if threaded {
        quote! { if my_perl.is_null() { return; } }
    } else {
        quote! {
            #[allow(unused_variables)]
            let my_perl: *mut ::libperl_rs::PerlInterpreter = ::core::ptr::null_mut();
        }
    };

    // (c) write to markstack_ptr (POPMARK)
    let pop_mark = if threaded {
        quote! {
            unsafe {
                (*my_perl).Imarkstack_ptr = (*my_perl).Imarkstack_ptr.sub(1);
            }
        }
    } else {
        quote! {
            unsafe {
                ::libperl_rs::PL_markstack_ptr = ::libperl_rs::PL_markstack_ptr.sub(1);
            }
        }
    };

    // (d) SP writer closure body
    let sp_writer = if threaded {
        quote! {
            let __set_sp_for_n = |n: usize| unsafe {
                (*my_perl).Istack_sp =
                    ::libperl_rs::PL_stack_base!(my_perl).add(__ax + n - 1);
            };
        }
    } else {
        quote! {
            let __set_sp_for_n = |n: usize| unsafe {
                ::libperl_rs::PL_stack_sp =
                    ::libperl_rs::PL_stack_base!(my_perl).add(__ax + n - 1);
            };
        }
    };

    // (e) per-argument extraction
    let arg_extractions: Vec<TokenStream2> = arg_specs
        .iter()
        .enumerate()
        .map(|(i, spec)| {
            let name = &spec.name;
            let svp_ident = quote::format_ident!("__svp_{}", name);
            let i_lit = syn::Index::from(i);
            // Common preamble: capture the SV pointer so we can later
            // write back (out-params) or pin its lifetime (string args).
            let svp_capture = quote! {
                let #svp_ident: *mut *mut ::libperl_rs::SV = unsafe {
                    ::libperl_rs::PL_stack_base!(my_perl).add(__ax + #i_lit)
                };
            };
            match spec.kind {
                ArgKind::In(flavor) | ArgKind::Out(flavor) => {
                    let is_mut = matches!(spec.kind, ArgKind::Out(_));
                    let (rust_ty, reader) = sv_flavor_input(flavor);
                    let mut_kw = if is_mut { quote! { mut } } else { quote! {} };
                    quote! {
                        #svp_capture
                        let #mut_kw #name: #rust_ty = unsafe {
                            ::libperl_rs::#reader(#myperl_arg_prefix *#svp_ident)
                        };
                    }
                }
                ArgKind::InCStr | ArgKind::InStr => {
                    let pv_ident = quote::format_ident!("__pv_{}", name);
                    let len_ident = quote::format_ident!("__pvlen_{}", name);
                    let cstr_ident = quote::format_ident!("__cstr_{}", name);
                    // SvPV-equivalent: handles GET magic, returns a
                    // NUL-terminated pointer (Perl always nul-terminates
                    // SV PV buffers) and writes the length to `lp`.
                    let extract_pv = quote! {
                        #svp_capture
                        let mut #len_ident: ::libperl_rs::STRLEN = 0;
                        let #pv_ident: *const ::core::ffi::c_char = unsafe {
                            ::libperl_rs::Perl_sv_2pv_flags(
                                #myperl_arg_prefix
                                *#svp_ident,
                                &mut #len_ident,
                                ::libperl_rs::SV_GMAGIC,
                            )
                        };
                        // Borrow the buffer as &CStr — same lifetime as
                        // the SV's PV, which lasts at least until this
                        // function returns.
                        let #cstr_ident: &::core::ffi::CStr =
                            unsafe { ::core::ffi::CStr::from_ptr(#pv_ident) };
                    };
                    if matches!(spec.kind, ArgKind::InCStr) {
                        quote! {
                            #extract_pv
                            let #name: &::core::ffi::CStr = #cstr_ident;
                        }
                    } else {
                        // &str — UTF-8 validate and croak on failure.
                        let usage_err_lit = syn::LitCStr::new(
                            std::ffi::CString::new(format!(
                                "argument `{}` is not valid UTF-8",
                                name
                            ))
                            .unwrap()
                            .as_c_str(),
                            proc_macro2::Span::call_site(),
                        );
                        quote! {
                            #extract_pv
                            let #name: &str = match #cstr_ident.to_str() {
                                ::core::result::Result::Ok(s) => s,
                                ::core::result::Result::Err(_) => {
                                    unsafe {
                                        ::libperl_rs::Perl_croak(
                                            #myperl_arg_prefix
                                            #usage_err_lit.as_ptr(),
                                        );
                                    }
                                }
                            };
                        }
                    }
                }
            }
        })
        .collect();

    // (f) out-param write-back (after body call, before return push)
    let out_writebacks: Vec<TokenStream2> = arg_specs
        .iter()
        .filter_map(|spec| {
            let ArgKind::Out(flavor) = spec.kind else { return None; };
            let name = &spec.name;
            let svp_ident = quote::format_ident!("__svp_{}", name);
            let setter = sv_flavor_setter(flavor);
            Some(quote! {
                unsafe {
                    ::libperl_rs::#setter(#myperl_arg_prefix *#svp_ident, #name);
                }
            })
        })
        .collect();

    // (g) return value push
    let return_push: TokenStream2 = match ret_kind {
        RetKind::Scalar(flavor) => {
            let setter = sv_flavor_setter(flavor);
            quote! {
                let __targ = unsafe { ::libperl_rs::Perl_sv_newmortal(#myperl_arg_only) };
                unsafe { ::libperl_rs::#setter(#myperl_arg_prefix __targ, __ret); }
                unsafe {
                // ST(0) = __targ, *not* `*__sp = __targ` — `__sp` was
                // captured *before* args were popped, so for items > 1
                // it points past ST(0). The right slot is base[ax+0].
                *::libperl_rs::PL_stack_base!(my_perl).add(__ax + 0) = __targ;
            }
                __set_sp_for_n(1);
            }
        }
        RetKind::Bool => quote! {
            let __targ = unsafe { ::libperl_rs::Perl_sv_newmortal(#myperl_arg_only) };
            unsafe {
                ::libperl_rs::Perl_sv_setiv(
                    #myperl_arg_prefix __targ,
                    __ret as ::libperl_rs::IV,
                );
            }
            unsafe {
                // ST(0) = __targ, *not* `*__sp = __targ` — `__sp` was
                // captured *before* args were popped, so for items > 1
                // it points past ST(0). The right slot is base[ax+0].
                *::libperl_rs::PL_stack_base!(my_perl).add(__ax + 0) = __targ;
            }
            __set_sp_for_n(1);
        },
        RetKind::String_ => quote! {
            let __targ = unsafe { ::libperl_rs::Perl_sv_newmortal(#myperl_arg_only) };
            // `String` always holds valid UTF-8, so we set the SV's
            // UTF8 flag too. `STRLEN` is `usize` on modern Perl and
            // `IV` (long) on older ones — `as _` lets rustc pick.
            let __bytes: &[u8] = __ret.as_bytes();
            unsafe {
                ::libperl_rs::Perl_sv_setpvn(
                    #myperl_arg_prefix
                    __targ,
                    __bytes.as_ptr() as *const ::core::ffi::c_char,
                    __bytes.len() as _,
                );
                // Mark UTF-8 by setting `SVf_UTF8` on the SV's flags
                // field. We can't call `SvUTF8_on` (the macrogen-
                // emitted one uses `sv_flags |= SVf_UTF8` which fails
                // to compile on Perls where `sv_flags` is `I32` while
                // `SVf_UTF8` is `u32` — see CI #25270605528). Round-
                // trip through `i64` (wide enough for either width)
                // and let the assignment infer the final type.
                let __cur_flags: i64 = (*__targ).sv_flags as i64;
                let __new_flags: i64 = __cur_flags | (::libperl_rs::SVf_UTF8 as i64);
                (*__targ).sv_flags = __new_flags as _;
            }
            unsafe {
                // ST(0) = __targ, *not* `*__sp = __targ` — `__sp` was
                // captured *before* args were popped, so for items > 1
                // it points past ST(0). The right slot is base[ax+0].
                *::libperl_rs::PL_stack_base!(my_perl).add(__ax + 0) = __targ;
            }
            __set_sp_for_n(1);
        },
        RetKind::Unit => quote! {
            let _ = __ret;
            __set_sp_for_n(0);
        },
    };

    // Note: NO `#[unsafe(no_mangle)]` here. The trampoline is referenced
    // only by function-pointer (passed to `Perl_newXS_deffile` from
    // `xs_boot!`), so it doesn't need a stable C symbol name. Worse,
    // *adding* `no_mangle` is actively harmful when the user's chosen
    // sub name happens to collide with a libc / libm symbol — e.g.
    // `fn round(arg: &mut NV)` exports a `round` symbol, which the
    // dynamic linker happily resolves to libm's `double round(double)`
    // when `boot_Mytest` looks up `Some(round)`. The XS sub then never
    // runs, libm's `round` is called with garbage arguments, and the
    // SV is never modified — silent failure.
    let trampoline = quote! {
        #body_fn

        #[allow(unused_variables, unreachable_code)]
        pub extern "C" fn #fn_name( #trampoline_params ) {
            #null_check

            // dXSARGS-equivalent: pop the mark, derive ax / items / sp.
            // Don't pin the offset type — `Stack_off_t` was introduced in
            // Perl 5.36; older Perls expose the same field as `SSize_t` /
            // `I32`. Letting rustc infer keeps the macro portable.
            let __sp: *mut *mut ::libperl_rs::SV = ::libperl_rs::PL_stack_sp!(my_perl);
            let __mark_ax = unsafe {
                *::libperl_rs::PL_markstack_ptr!(my_perl)
            };
            #pop_mark
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

            // Argument extraction (also remembers svp pointers for any
            // out-params, so we can write the new value back at the end).
            #( #arg_extractions )*

            // User body — invoked via the hidden helper fn so the user's
            // typed signature is preserved for tooling and import-usage
            // analysis.
            let __ret: #ret_ty_for_user = #body_fn_name( #( #user_arg_call ),* );

            // Out-param write-back (`OUTPUT: arg` in xsubpp parlance).
            #( #out_writebacks )*

            // SP setter helper, centralised so the off-by-one is not at
            // each call site
            #sp_writer

            // Return value push
            #return_push
        }
    };

    trampoline.into()
}

// ─── Type-classifier helpers ───────────────────────────────────────

fn classify_arg_type(ty: &Type) -> Option<ArgKind> {
    if let Type::Reference(TypeReference { mutability, elem, .. }) = ty {
        if mutability.is_some() {
            // `&mut T` — out-parameter (only scalar flavors supported).
            return classify_scalar(elem).map(ArgKind::Out);
        }
        // `&T` — currently only `&str` and `&CStr` are recognised.
        if let Type::Path(TypePath { path, .. }) = elem.as_ref() {
            let last = path.segments.last()?;
            return match last.ident.to_string().as_str() {
                "str" => Some(ArgKind::InStr),
                "CStr" => Some(ArgKind::InCStr),
                _ => None,
            };
        }
        return None;
    }
    classify_scalar(ty).map(ArgKind::In)
}

fn classify_ret_type(ty: &Type) -> Option<RetKind> {
    if let Type::Tuple(t) = ty {
        if t.elems.is_empty() {
            return Some(RetKind::Unit);
        }
    }
    if let Type::Path(TypePath { path, .. }) = ty {
        let last = path.segments.last()?;
        match last.ident.to_string().as_str() {
            "bool" => return Some(RetKind::Bool),
            "String" => return Some(RetKind::String_),
            _ => {}
        }
    }
    classify_scalar(ty).map(RetKind::Scalar)
}

fn classify_scalar(ty: &Type) -> Option<SvFlavor> {
    if let Type::Path(TypePath { path, .. }) = ty {
        let last = path.segments.last()?;
        return match last.ident.to_string().as_str() {
            "IV" => Some(SvFlavor::Iv),
            "UV" => Some(SvFlavor::Uv),
            "NV" => Some(SvFlavor::Nv),
            _ => None,
        };
    }
    None
}

/// `(Rust scalar type, Sv* reader fn ident)` for the given flavor.
fn sv_flavor_input(flavor: SvFlavor) -> (TokenStream2, Ident) {
    match flavor {
        SvFlavor::Iv => (
            quote! { ::libperl_rs::IV },
            Ident::new("SvIV", proc_macro2::Span::call_site()),
        ),
        SvFlavor::Uv => (
            quote! { ::libperl_rs::UV },
            Ident::new("SvUV", proc_macro2::Span::call_site()),
        ),
        SvFlavor::Nv => (
            quote! { ::libperl_rs::NV },
            Ident::new("SvNV", proc_macro2::Span::call_site()),
        ),
    }
}

/// `Perl_sv_set*` ident for the given flavor.
fn sv_flavor_setter(flavor: SvFlavor) -> Ident {
    let n = match flavor {
        SvFlavor::Iv => "Perl_sv_setiv",
        SvFlavor::Uv => "Perl_sv_setuv",
        SvFlavor::Nv => "Perl_sv_setnv",
    };
    Ident::new(n, proc_macro2::Span::call_site())
}

fn error<T: quote::ToTokens>(spanned: T, msg: &str) -> TokenStream {
    syn::Error::new_spanned(spanned, msg).to_compile_error().into()
}
