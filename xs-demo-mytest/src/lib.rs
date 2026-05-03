//! Mytest — minimal Rust XS module demonstrating `#[xs_sub]`.
//!
//! Mirrors the hand-written `is_even` example from
//! <https://github.com/hkoba/exp-libperl-rs-xs1>, but generated from a
//! one-line Rust signature.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libperl_rs::{xs_boot, xs_sub, IV};

/// `Mytest::is_even($n)` — returns true if `$n` is even.
#[xs_sub]
fn is_even(n: IV) -> bool {
    n % 2 == 0
}

xs_boot! {
    package = "Mytest";
    subs = [is_even];
}
