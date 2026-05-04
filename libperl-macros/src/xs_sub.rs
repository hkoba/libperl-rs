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
    /// `*mut SV` — raw pass-through of the caller's SV pointer.
    /// No conversion, no magic, no refcount fiddling. The user body
    /// is responsible for any further unpacking.
    InRawSv,
    /// `Sv` — `NonNull<SV>` newtype wrapper. Same ABI as `*mut SV`
    /// but encodes the non-null invariant in the type. Constructed
    /// via `Sv::from_raw_unchecked` from the stack pointer.
    InSv,
    /// `&Perl` — explicit interpreter context (Phase 3.10c). Does NOT
    /// consume a Perl-side stack slot; the trampoline materializes a
    /// borrowed `Perl` from `my_perl` and passes a reference to it.
    /// By convention this is the first parameter and is named
    /// `my_perl` (see `docs/plan/README.md` §3.8).
    PerlContext,
}

#[derive(Clone)]
enum RetKind {
    Scalar(SvFlavor),
    Bool,
    Unit,
    /// `String` — the bytes are pushed via `Perl_sv_setpvn`.
    String_,
    /// `Vec<IV / UV / NV>` — variable-length list of scalar SVs.
    /// PPCODE-style: extend the stack, push each mortal value.
    VecScalar(SvFlavor),
    /// `Vec<String>` — variable-length list of UTF-8 string SVs.
    VecString,
    /// `Result<T, String>` — `Ok(v)` proceeds with the inner kind's
    /// push path; `Err(s)` calls `Perl_croak` with `s` as the message.
    ResultErrString(Box<RetKind>),
    /// `*mut SV` — raw pass-through of an SV pointer the body
    /// produced. The trampoline `SvREFCNT_inc`s + mortalizes (the
    /// XS T_SV typemap convention) so that newly-created and
    /// borrowed SVs both behave correctly.
    RawSv,
    /// `Option<*mut SV>` — `Some(sv)` behaves like `RawSv`; `None`
    /// pushes `PL_sv_undef` (immortal global, no refcount fiddling).
    /// XSRETURN_UNDEF equivalent in idiomatic Rust.
    OptionRawSv,
    /// `Sv` — `NonNull<SV>` newtype. Push behaviour identical to
    /// `RawSv` (T_SV typemap convention) — the newtype just encodes
    /// the non-null invariant in the type system.
    Sv,
    /// `Option<Sv>` — `Some` like `Sv`, `None` like `OptionRawSv`'s
    /// undef path.
    OptionSv,
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
                             / `&mut IV|UV|NV` / `&CStr` / `&str` / `*mut SV` \
                             / `Sv` / `&Perl`",
                        );
                    }
                };
                // `&Perl` must be the first parameter — naming
                // convention §3.8 and to keep the trampoline simple.
                if matches!(kind, ArgKind::PerlContext) && !arg_specs.is_empty() {
                    return error(
                        ty,
                        "`&Perl` (interpreter context) must be the first \
                         parameter of an `#[xs_sub]`",
                    );
                }
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
    // everything else passes the local binding by value.
    let user_arg_call: Vec<TokenStream2> = arg_specs
        .iter()
        .map(|s| {
            let n = &s.name;
            match s.kind {
                ArgKind::In(_)
                | ArgKind::InCStr
                | ArgKind::InStr
                | ArgKind::InRawSv
                | ArgKind::InSv => quote! { #n },
                ArgKind::Out(_) => quote! { &mut #n },
                // The trampoline injects a `__perl_ref: &Perl` local
                // before calling the body fn (see `perl_ref_setup`).
                ArgKind::PerlContext => quote! { __perl_ref },
            }
        })
        .collect();

    // Stack-arg count (PerlContext does not consume a slot). Used both
    // for the arity check and for per-arg stack-index assignment below.
    let arg_count = arg_specs
        .iter()
        .filter(|s| !matches!(s.kind, ArgKind::PerlContext))
        .count();
    let usage_str = arg_specs
        .iter()
        .filter(|a| !matches!(a.kind, ArgKind::PerlContext))
        .map(|a| a.name.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let needs_perl_ref = arg_specs
        .iter()
        .any(|s| matches!(s.kind, ArgKind::PerlContext));

    // ManuallyDrop wrapper because `Perl::from_raw_unchecked` produces
    // a `Perl` whose `Drop` would call `perl_destruct` — we don't own
    // the interpreter here, so dropping must be suppressed.
    let perl_ref_setup: TokenStream2 = if needs_perl_ref {
        quote! {
            let __perl_ctx_storage = ::core::mem::ManuallyDrop::new(unsafe {
                ::libperl_rs::Perl::from_raw_unchecked(my_perl)
            });
            let __perl_ref: &::libperl_rs::Perl = &*__perl_ctx_storage;
        }
    } else {
        quote! {}
    };
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
            // `move` so the closure copies `my_perl` (it's `Copy`)
            // rather than borrowing it — otherwise later raw-place
            // expressions like `&raw mut (*my_perl).Isv_undef` would
            // collide with the closure's outstanding borrow.
            let __set_sp_for_n = move |n: usize| unsafe {
                (*my_perl).Istack_sp =
                    ::libperl_rs::PL_stack_base!(my_perl).add(__ax + n - 1);
            };
        }
    } else {
        quote! {
            // `move` so the closure copies `my_perl` (it's `Copy`)
            // rather than borrowing it — otherwise later raw-place
            // expressions like `&raw mut (*my_perl).Isv_undef` would
            // collide with the closure's outstanding borrow.
            let __set_sp_for_n = move |n: usize| unsafe {
                ::libperl_rs::PL_stack_sp =
                    ::libperl_rs::PL_stack_base!(my_perl).add(__ax + n - 1);
            };
        }
    };

    // (e) per-argument extraction. Stack-slot index advances only for
    // args that actually consume a Perl-side slot — `PerlContext` does
    // not, so it gets a no-op extraction.
    let mut __stack_idx: usize = 0;
    let arg_extractions: Vec<TokenStream2> = arg_specs
        .iter()
        .map(|spec| {
            if matches!(spec.kind, ArgKind::PerlContext) {
                return quote! {};
            }
            let name = &spec.name;
            let svp_ident = quote::format_ident!("__svp_{}", name);
            let i_lit = syn::Index::from(__stack_idx);
            __stack_idx += 1;
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
                ArgKind::InRawSv => {
                    // Pass through the raw SV pointer; no Sv* call.
                    quote! {
                        #svp_capture
                        let #name: *mut ::libperl_rs::SV = unsafe { *#svp_ident };
                    }
                }
                ArgKind::InSv => {
                    // Wrap the stack SV in the `Sv` newtype. Pointers
                    // pulled off the Perl stack are always non-null
                    // (the arity check prevents the slot being absent).
                    quote! {
                        #svp_capture
                        let #name: ::libperl_rs::Sv = unsafe {
                            ::libperl_rs::Sv::from_raw_unchecked(*#svp_ident)
                        };
                    }
                }
                ArgKind::PerlContext => unreachable!("handled by early return above"),
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
                                // `flags` parameter is `U32` on modern
                                // Perl but `I32` on 5.30 / 5.32 — `as _`
                                // lets rustc infer the right width.
                                ::libperl_rs::SV_GMAGIC as _,
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

    // (g) Result wrapper: emit unwrap-or-croak, then continue with the
    // inner kind's push code. For non-Result returns, this is a no-op.
    let (unwrap_code, push_kind) = match ret_kind {
        RetKind::ResultErrString(inner) => (
            quote! {
                // `Perl_croak` is variadic and returns `!`. We pass
                // `"%s\n"` plus a NUL-terminated message; if the
                // user-supplied error text contains an interior NUL,
                // fall back to a fixed warning so we still croak
                // safely instead of panicking.
                let __ret = match __raw {
                    ::core::result::Result::Ok(v) => v,
                    ::core::result::Result::Err(__e) => {
                        let __msg = ::std::ffi::CString::new(__e)
                            .unwrap_or_else(|_| ::std::ffi::CString::new(
                                "xs_sub: error message contained interior NUL",
                            ).unwrap());
                        unsafe {
                            ::libperl_rs::Perl_croak(
                                #myperl_arg_prefix
                                c"%s\n".as_ptr(),
                                __msg.as_ptr(),
                            );
                        }
                    }
                };
            },
            *inner,
        ),
        other => (quote! { let __ret = __raw; }, other),
    };

    // (h) push code per inner RetKind
    let push_code: TokenStream2 = match push_kind {
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
        RetKind::VecScalar(flavor) => {
            let setter = sv_flavor_setter(flavor);
            quote! {
                let __n: usize = __ret.len();
                // Make sure the stack can hold __n return slots.
                // `Perl_stack_grow` is a no-op when there's already
                // room; otherwise it reallocates (the new sp is
                // returned, but PL_stack_base updates internally).
                unsafe {
                    let _ = ::libperl_rs::Perl_stack_grow(
                        #myperl_arg_prefix
                        ::libperl_rs::PL_stack_sp!(my_perl),
                        ::libperl_rs::PL_stack_sp!(my_perl),
                        __n as _,
                    );
                }
                for (__i, __val) in __ret.iter().enumerate() {
                    unsafe {
                        let __targ = ::libperl_rs::Perl_sv_newmortal(#myperl_arg_only);
                        ::libperl_rs::#setter(#myperl_arg_prefix __targ, *__val);
                        *::libperl_rs::PL_stack_base!(my_perl).add(__ax + __i) = __targ;
                    }
                }
                __set_sp_for_n(__n);
            }
        }
        RetKind::VecString => quote! {
            let __n: usize = __ret.len();
            unsafe {
                let _ = ::libperl_rs::Perl_stack_grow(
                    #myperl_arg_prefix
                    ::libperl_rs::PL_stack_sp!(my_perl),
                    ::libperl_rs::PL_stack_sp!(my_perl),
                    __n as _,
                );
            }
            for (__i, __val) in __ret.iter().enumerate() {
                unsafe {
                    let __targ = ::libperl_rs::Perl_sv_newmortal(#myperl_arg_only);
                    let __bytes: &[u8] = __val.as_bytes();
                    ::libperl_rs::Perl_sv_setpvn(
                        #myperl_arg_prefix
                        __targ,
                        __bytes.as_ptr() as *const ::core::ffi::c_char,
                        __bytes.len() as _,
                    );
                    let __cur_flags: i64 = (*__targ).sv_flags as i64;
                    (*__targ).sv_flags =
                        (__cur_flags | (::libperl_rs::SVf_UTF8 as i64)) as _;
                    *::libperl_rs::PL_stack_base!(my_perl).add(__ax + __i) = __targ;
                }
            }
            __set_sp_for_n(__n);
        },
        RetKind::ResultErrString(_) => unreachable!("Result is unwrapped before push"),
        RetKind::RawSv => quote! {
            // T_SV typemap convention: refcount-inc the SV (so caller-
            // owned and freshly-created SVs both behave) and put it on
            // the mortal stack so it's freed at end of expression.
            let __sv: *mut ::libperl_rs::SV = __ret;
            unsafe {
                let __mortal = ::libperl_rs::Perl_sv_2mortal(
                    #myperl_arg_prefix
                    ::libperl_rs::sv_refcnt_inc(__sv),
                );
                *::libperl_rs::PL_stack_base!(my_perl).add(__ax + 0) = __mortal;
            }
            __set_sp_for_n(1);
        },
        RetKind::Sv => quote! {
            // Same as RawSv but unwrap the newtype first.
            let __sv: *mut ::libperl_rs::SV = __ret.as_ptr();
            unsafe {
                let __mortal = ::libperl_rs::Perl_sv_2mortal(
                    #myperl_arg_prefix
                    ::libperl_rs::sv_refcnt_inc(__sv),
                );
                *::libperl_rs::PL_stack_base!(my_perl).add(__ax + 0) = __mortal;
            }
            __set_sp_for_n(1);
        },
        RetKind::OptionSv => quote! {
            let __pushed: *mut ::libperl_rs::SV = match __ret {
                ::core::option::Option::Some(__sv_wrap) => unsafe {
                    ::libperl_rs::Perl_sv_2mortal(
                        #myperl_arg_prefix
                        ::libperl_rs::sv_refcnt_inc(__sv_wrap.as_ptr()),
                    )
                },
                ::core::option::Option::None => ::libperl_rs::sv_undef_ptr(my_perl),
            };
            unsafe {
                *::libperl_rs::PL_stack_base!(my_perl).add(__ax + 0) = __pushed;
            }
            __set_sp_for_n(1);
        },
        RetKind::OptionRawSv => {
            // `PL_sv_undef`'s storage location differs across build
            // modes (and across Perl versions in non-threaded), so we
            // delegate to the `sv_undef_ptr` helper in libperl-rs.
            quote! {
                let __pushed: *mut ::libperl_rs::SV = match __ret {
                    ::core::option::Option::Some(__sv) => unsafe {
                        ::libperl_rs::Perl_sv_2mortal(
                            #myperl_arg_prefix
                            ::libperl_rs::sv_refcnt_inc(__sv),
                        )
                    },
                    // `PL_sv_undef` is immortal — no INC / mortalize.
                    ::core::option::Option::None => ::libperl_rs::sv_undef_ptr(my_perl),
                };
                unsafe {
                    *::libperl_rs::PL_stack_base!(my_perl).add(__ax + 0) = __pushed;
                }
                __set_sp_for_n(1);
            }
        }
    };

    let return_push: TokenStream2 = quote! {
        #unwrap_code
        #push_code
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

            // Materialize the borrowed `Perl` context if any arg needs
            // it. Wrapped in `ManuallyDrop` so we don't tear down an
            // interpreter we don't own.
            #perl_ref_setup

            // User body — invoked via the hidden helper fn so the user's
            // typed signature is preserved for tooling and import-usage
            // analysis. Bound as `__raw` so the Result-unwrap layer
            // (when present) can produce `__ret` from it.
            let __raw: #ret_ty_for_user = #body_fn_name( #( #user_arg_call ),* );

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
        // `&T` — `&str`, `&CStr`, or `&Perl` (the interpreter context).
        if let Type::Path(TypePath { path, .. }) = elem.as_ref() {
            let last = path.segments.last()?;
            return match last.ident.to_string().as_str() {
                "str" => Some(ArgKind::InStr),
                "CStr" => Some(ArgKind::InCStr),
                "Perl" => Some(ArgKind::PerlContext),
                _ => None,
            };
        }
        return None;
    }
    if is_raw_sv_ptr(ty) {
        return Some(ArgKind::InRawSv);
    }
    if is_sv_newtype(ty) {
        return Some(ArgKind::InSv);
    }
    classify_scalar(ty).map(ArgKind::In)
}

fn classify_ret_type(ty: &Type) -> Option<RetKind> {
    if let Type::Tuple(t) = ty {
        if t.elems.is_empty() {
            return Some(RetKind::Unit);
        }
    }
    if is_raw_sv_ptr(ty) {
        return Some(RetKind::RawSv);
    }
    if is_sv_newtype(ty) {
        return Some(RetKind::Sv);
    }
    if is_rv_wrapper(ty) {
        // `Rv<T>` is shaped exactly like `Sv` from the trampoline's
        // perspective — it has an `as_ptr() -> *mut SV` method and the
        // SV underneath is already mortal (constructed via `into_rv`).
        return Some(RetKind::Sv);
    }
    if let Type::Path(TypePath { path, .. }) = ty {
        let last = path.segments.last()?;
        match last.ident.to_string().as_str() {
            "bool" => return Some(RetKind::Bool),
            "String" => return Some(RetKind::String_),
            "Vec" => return classify_vec_inner(&last.arguments),
            "Result" => return classify_result_inner(&last.arguments),
            "Option" => return classify_option_inner(&last.arguments),
            _ => {}
        }
    }
    classify_scalar(ty).map(RetKind::Scalar)
}

/// `Option<T>` — `Option<*mut SV>` (Phase 3.10a), `Option<Sv>`
/// (Phase 3.10b), `Option<Rv<U>>` (Phase 3.10c). All wrap the same
/// "Some pushes the SV, None pushes undef" pattern; the inner type
/// just selects which `as_ptr()` is in scope.
fn classify_option_inner(args: &syn::PathArguments) -> Option<RetKind> {
    let inner = generic_arg_n(args, 0)?;
    if is_raw_sv_ptr(inner) {
        return Some(RetKind::OptionRawSv);
    }
    if is_sv_newtype(inner) || is_rv_wrapper(inner) {
        return Some(RetKind::OptionSv);
    }
    None
}

/// True when `ty` is `*mut SV` (path's last segment is `SV`,
/// regardless of leading `::libperl_rs::` qualification).
fn is_raw_sv_ptr(ty: &Type) -> bool {
    let Type::Ptr(p) = ty else { return false };
    if p.const_token.is_some() || p.mutability.is_none() {
        return false;
    }
    let Type::Path(TypePath { path, .. }) = p.elem.as_ref() else {
        return false;
    };
    path.segments.last().is_some_and(|s| s.ident == "SV")
}

/// True when `ty` is the `Sv` newtype (path's last segment is `Sv`,
/// e.g. `Sv` or `libperl_rs::Sv`).
fn is_sv_newtype(ty: &Type) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else { return false };
    path.segments.last().is_some_and(|s| s.ident == "Sv" && s.arguments.is_none())
}

/// True when `ty` is `Rv<...>` — any single-generic `Rv` path
/// (`Rv<Av>`, `libperl_rs::Rv<Hv>`, etc.). The proc-macro doesn't
/// inspect the generic arg; the body fn's type system enforces what's
/// actually inside.
fn is_rv_wrapper(ty: &Type) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else { return false };
    let Some(last) = path.segments.last() else { return false };
    if last.ident != "Rv" {
        return false;
    }
    matches!(last.arguments, syn::PathArguments::AngleBracketed(_))
}

/// `Vec<T>` — inspect the single generic arg.
fn classify_vec_inner(args: &syn::PathArguments) -> Option<RetKind> {
    let inner = generic_arg_n(args, 0)?;
    if let Type::Path(TypePath { path, .. }) = inner {
        if path.segments.last().is_some_and(|s| s.ident == "String") {
            return Some(RetKind::VecString);
        }
    }
    classify_scalar(inner).map(RetKind::VecScalar)
}

/// `Result<T, String>` — recurse into `T`. The error type is required
/// to be `String` (any path whose last segment is `String`).
fn classify_result_inner(args: &syn::PathArguments) -> Option<RetKind> {
    let ok_ty = generic_arg_n(args, 0)?;
    let err_ty = generic_arg_n(args, 1)?;
    let err_is_string = matches!(
        err_ty,
        Type::Path(TypePath { path, .. })
            if path.segments.last().is_some_and(|s| s.ident == "String"),
    );
    if !err_is_string {
        return None;
    }
    let inner = classify_ret_type(ok_ty)?;
    Some(RetKind::ResultErrString(Box::new(inner)))
}

fn generic_arg_n(args: &syn::PathArguments, n: usize) -> Option<&Type> {
    let syn::PathArguments::AngleBracketed(args) = args else { return None };
    let arg = args.args.iter().nth(n)?;
    let syn::GenericArgument::Type(ty) = arg else { return None };
    Some(ty)
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
