#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unnecessary_transmutes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(perlapi_ver40)]
pub type perl_stack_size_t = isize;

#[cfg(not(perlapi_ver40))]
pub type perl_stack_size_t = i32;
