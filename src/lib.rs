//! libperl-rs — safe Rust API on top of libperl-sys / libperl-macrogen.
//!
//! This crate is being rebuilt on top of libperl-macrogen (see
//! `docs/plan/README.md`). The previous prototype API has been moved to the
//! `libperl-proto0` workspace member.
//!
//! The new safe API is currently empty; macrogen-generated FFI is re-exported
//! via `libperl_sys` for now.

pub use libperl_sys;
