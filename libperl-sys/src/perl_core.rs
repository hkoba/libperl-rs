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
