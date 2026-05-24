extern crate libperl_sys;

#[allow(unused_imports)]
#[macro_use]
extern crate if_chain; // For OpExtractor

pub mod perl;
pub use perl::*;

// Tests live in `tests/parse_errors.rs` so that they can capture the
// real fd-2 stderr (Perl writes diagnostics via the C runtime, not via
// Rust's `eprintln!`). See that file for the assertions on
// `Perl::parse` failure and success paths.
