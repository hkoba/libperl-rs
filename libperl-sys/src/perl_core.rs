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
