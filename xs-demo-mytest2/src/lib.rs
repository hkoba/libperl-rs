//! Mytest2 — demo of `#[xs_sub]` Phase 3.8 features:
//! `&CStr` / `&str` arguments and `String` / `NV` return values
//! (perlxstut EXAMPLE 4 territory).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::CStr;

use libperl_rs::{xs_boot, xs_sub, IV, NV};

/// `Mytest2::foo($i, $l, $str)` — perlxstut EXAMPLE 4 shape.
///
/// The original tutorial wraps an external C library. Here we just do
/// `i + l + length($str)` so the test exercises the type machinery
/// without pulling in an extra C dependency.
#[xs_sub]
fn foo(i: IV, l: IV, s: &CStr) -> NV {
    (i + l + s.to_bytes().len() as IV) as NV
}

/// String → String round-trip (uppercase). Exercises `&str` + UTF-8
/// validation on the input and `String` return on the output.
#[xs_sub]
fn shout(input: &str) -> String {
    input.to_uppercase()
}

/// String → IV (length in bytes). Demonstrates `&CStr` input + IV
/// return (no UTF-8 checks).
#[xs_sub]
fn byte_len(s: &CStr) -> IV {
    s.to_bytes().len() as IV
}

xs_boot! {
    package = "Mytest2";
    subs = [foo, shout, byte_len];
}
