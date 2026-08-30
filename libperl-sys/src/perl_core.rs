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
