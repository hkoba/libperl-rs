//! Mytest — minimal Rust XS module demonstrating `#[xs_sub]`.
//!
//! Mirrors the hand-written `is_even` example from
//! <https://github.com/hkoba/exp-libperl-rs-xs1>, but generated from a
//! one-line Rust signature.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libperl_rs::{xs_boot, xs_sub, IV, NV};

/// `Mytest::is_even($n)` — returns true if `$n` is even.
/// (perlxstut EXAMPLE 2.)
#[xs_sub]
fn is_even(n: IV) -> bool {
    n % 2 == 0
}

/// `Mytest::round($x)` — round `$x` to the nearest integer, in place.
/// Out-parameter via `&mut NV`.  (perlxstut EXAMPLE 3.)
#[xs_sub]
fn round(arg: &mut NV) {
    if *arg > 0.0 {
        *arg = (*arg + 0.5).floor();
    } else if *arg < 0.0 {
        *arg = (*arg - 0.5).ceil();
    } else {
        *arg = 0.0;
    }
}

xs_boot! {
    package = "Mytest";
    subs = [is_even, round];
}
