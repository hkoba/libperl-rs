//! Auto-generated FFI declarations (bindgen + libperl-macrogen output).
//!
#![doc = concat!(
    "Built against Perl ", env!("LIBPERL_SYS_PERL_VERSION"),
    " (", env!("LIBPERL_SYS_PERL_THREADED"),
    ", `", env!("LIBPERL_SYS_PERL_ARCHNAME"), "`).",
)]
//!
//! See the [crate root](crate) for build-target details and version
//! constants.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unnecessary_transmutes)]
#![allow(unpredictable_function_pointer_comparisons)]
#![allow(unused_imports)]
#![allow(unused_mut)]
// 5.20 の生成体 (SvSCREAM_on 族の sv_flags |= (...)) が unused_parens を
// 3 件出す。下流が -D warnings でビルドしても生成コード起因で落ちない
// よう、他の生成コード用 allow と同列で許容しておく。
#![allow(unused_parens)]
// 5.24 の生成体 (CvDEPTH デクリメント族の
// `{ *S_CvDEPTHp(cv) -= 1; *S_CvDEPTHp(cv) }` 文) が unused_must_use を
// 出す (CI は RUSTFLAGS=-D warnings で error 化する)。同上の理由で許容。
#![allow(unused_must_use)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

include!(concat!(env!("OUT_DIR"), "/macro_bindings.rs"));

// GH-20: PERLVAR 由来の読み書き両用アクセサ `PL_xxx_ptr!` (build.rs 生成)。
// read 専用の `PL_xxx!` (macro_bindings.rs) と同じ PERLVAR 観測が情報源。
include!(concat!(env!("OUT_DIR"), "/perlvar_ptr_bindings.rs"));

#[cfg(perlapi_ver40)]
pub type perl_stack_size_t = isize;

#[cfg(not(perlapi_ver40))]
pub type perl_stack_size_t = i32;

// Compat: `Stack_off_t` first appeared in the perl 5.40 stack refactor;
// before that the mark stack was plain `I32`. Alias it on older perls so
// downstream code can use `Stack_off_t` uniformly across versions
// (bindgen emits the real typedef on 5.40+).
#[cfg(not(perlapi_ver40))]
pub type Stack_off_t = I32;

// Compat: perl 5.31.x renamed the inline refcount helper `S_SvREFCNT_dec`
// to `Perl_SvREFCNT_dec`, so the `Perl_` name only exists from 5.32 on.
// Below 5.32 macrogen generates `S_SvREFCNT_dec` with the identical
// signature (verified on 5.20-5.30 x both modes with libperl-macrogen
// 0.1.12 / apidoc data 1.15); alias it so downstream code can call
// `Perl_SvREFCNT_dec` uniformly across versions.
#[cfg(not(perlapi_ver32))]
pub use self::S_SvREFCNT_dec as Perl_SvREFCNT_dec;

// Compat: the `OpSIBLING` macro first appeared in perl 5.21.2. On 5.20
// mirror the old-world direct `op_sibling` read, with the signature
// matching the 5.22+ macrogen output (`fn OpSIBLING(o: *mut OP) -> *mut OP`).
#[cfg(not(perlapi_ver22))]
pub unsafe fn OpSIBLING(o: *mut OP) -> *mut OP {
    unsafe { (*o).op_sibling }
}

// Compat: `Perl_newXS_deffile` and `Perl_xs_boot_epilog` were both born
// in perl 5.21.x (first release: 5.22) as part of the XS bootstrap
// rework, and the `xs_boot!` proc-macro expansion references them. On
// 5.20 mirror the 5.22 implementations (op.c `newXS_deffile` / perl.c
// `xs_boot_epilog`), with signatures matching the 5.22 bindgen externs.
#[cfg(all(not(perlapi_ver22), perl_useithreads))]
pub unsafe fn Perl_newXS_deffile(
    my_perl: *mut PerlInterpreter,
    name: *const ::std::os::raw::c_char,
    subaddr: XSUBADDR_t,
) -> *mut CV {
    // 5.22's newXS_deffile passes file = NULL (leaving CvFILE unset);
    // 5.20's newXS asserts a filename, so hand it an empty string.
    unsafe { Perl_newXS(my_perl, name, subaddr, c"".as_ptr()) }
}

#[cfg(all(not(perlapi_ver22), not(perl_useithreads)))]
pub unsafe fn Perl_newXS_deffile(
    name: *const ::std::os::raw::c_char,
    subaddr: XSUBADDR_t,
) -> *mut CV {
    unsafe { Perl_newXS(name, subaddr, c"".as_ptr()) }
}

// 5.22 perl.c: run UNITCHECK blocks queued during boot, then
// XSRETURN_YES (ST(0) = &PL_sv_yes; XSRETURN(1)).
#[cfg(all(not(perlapi_ver22), perl_useithreads))]
pub unsafe fn Perl_xs_boot_epilog(my_perl: *mut PerlInterpreter, ax: U32) {
    unsafe {
        if !(*my_perl).Iunitcheckav.is_null() {
            Perl_call_list(my_perl, (*my_perl).Iscopestack_ix, (*my_perl).Iunitcheckav);
        }
        let slot = (*my_perl).Istack_base.add(ax as usize);
        *slot = &raw mut (*my_perl).Isv_yes;
        (*my_perl).Istack_sp = slot;
    }
}

#[cfg(all(not(perlapi_ver22), not(perl_useithreads)))]
pub unsafe fn Perl_xs_boot_epilog(ax: U32) {
    unsafe {
        if !PL_unitcheckav.is_null() {
            Perl_call_list(PL_scopestack_ix, PL_unitcheckav);
        }
        let slot = PL_stack_base.add(ax as usize);
        *slot = &raw mut PL_sv_yes;
        PL_stack_sp = slot;
    }
}
