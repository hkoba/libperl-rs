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
// On 5.28/5.30 macrogen generates `S_SvREFCNT_dec` with the identical
// signature (libperl-macrogen apidoc data 1.14); alias it so downstream
// code can call `Perl_SvREFCNT_dec` uniformly across versions. Gated to
// >= 5.28 because older perls don't generate `S_SvREFCNT_dec` yet.
#[cfg(all(perlapi_ver28, not(perlapi_ver32)))]
pub use self::S_SvREFCNT_dec as Perl_SvREFCNT_dec;

// Compat: on perl 5.28/5.30 the current libperl-macrogen apidoc data
// suppresses codegen for the Padname* / Padnamelist* accessors
// ("[CODEGEN_SUPPRESSED] (apidoc patch)"), so they are missing from
// macro_bindings.rs there. The underlying struct fields are identical
// to 5.32+, so mirror the bodies macrogen emits on 5.32-5.44 until a
// future unskip round lifts the suppression. `isize` matches the
// generated `ssize_t` return type (that libc alias itself is absent
// from the 5.28/5.30 bindgen output).
#[cfg(all(perlapi_ver28, not(perlapi_ver32)))]
pub unsafe fn PadnamelistMAX(pnl: *const PADNAMELIST) -> isize {
    unsafe { (*pnl).xpadnl_fill }
}

#[cfg(all(perlapi_ver28, not(perlapi_ver32)))]
pub unsafe fn PadnamelistARRAY(pnl: *const PADNAMELIST) -> *mut *mut PADNAME {
    unsafe { (*pnl).xpadnl_alloc }
}

#[cfg(all(perlapi_ver28, not(perlapi_ver32)))]
pub unsafe fn PadnamePV(pn: *mut PADNAME) -> *mut ::std::os::raw::c_char {
    unsafe { (*pn).xpadn_pv }
}

#[cfg(all(perlapi_ver28, not(perlapi_ver32)))]
pub unsafe fn PadnameLEN(pn: *mut PADNAME) -> STRLEN {
    unsafe { (*pn).xpadn_len as STRLEN }
}

#[cfg(all(perlapi_ver28, not(perlapi_ver32)))]
pub unsafe fn PadnameTYPE(pn: *mut PADNAME) -> *mut HV {
    unsafe { (*pn).xpadn_type_u.xpadn_typestash }
}
