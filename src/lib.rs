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

/// `thx_call!(perl, Perl_xxx, args...)` — call a libperl-sys function
/// that takes a leading `my_perl` parameter in threaded builds and
/// drops that parameter in non-threaded builds. The first argument
/// (`perl: &Perl`) is silently discarded in non-threaded mode.
///
/// Centralising this here keeps the hand-written `Sv`/`Av`/`Hv`
/// constructors free of `#[cfg(perl_useithreads)]` clutter — same
/// abstraction the `#[xs_sub]` proc-macro applies internally via
/// `myperl_arg_prefix`.
macro_rules! thx_call {
    ($perl:expr, $fn:ident, $($arg:expr),* $(,)?) => {{
        #[cfg(perl_useithreads)]
        { libperl_sys::$fn($perl.as_ptr(), $($arg),*) }
        #[cfg(not(perl_useithreads))]
        { let _ = $perl; libperl_sys::$fn($($arg),*) }
    }};
}
pub(crate) use thx_call;

pub mod perl;
pub use perl::*;

pub mod sv;
pub use sv::*;

pub mod cv;
pub use cv::*;

pub mod rv;
pub use rv::*;

pub mod av;
pub use av::*;

pub mod hv;
pub use hv::*;
