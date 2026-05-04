//! libperl-rs — safe Rust API on top of libperl-sys / libperl-macrogen.
//!
//! Re-exports everything from `libperl-sys` at the crate root so that
//! consumers can write `libperl_rs::Perl_sv_setiv(...)` and
//! `libperl_rs::PL_main_start!(my_perl)` uniformly. The old prototype API
//! lives on as the `libperl-proto0` workspace member; see
//! `docs/plan/README.md` for the rebuild plan.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

pub use libperl_sys::*;
pub use libperl_macros::*;

pub mod perl;
pub use perl::*;

pub mod sv;
pub use sv::*;

pub mod rv;
pub use rv::*;

pub mod av;
pub use av::*;

pub mod hv;
pub use hv::*;
