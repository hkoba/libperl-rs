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

/// `Mytest2::statfs($path)` — perlxstut EXAMPLE 5 shape.
///
/// On success returns a 7-element list `(bsize, frsize, blocks,
/// bfree, bavail, files, ffree)` from `statvfs(3)`. On failure
/// croaks with the OS error message (perlxstut returns a single NV
/// of `errno`; we use `Result::Err` so the caller sees `$@`
/// instead of a magic-number list).
#[xs_sub]
fn statfs(path: &CStr) -> Result<Vec<NV>, String> {
    let mut sb: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(path.as_ptr(), &mut sb) };
    if rc != 0 {
        return Err(format!(
            "statvfs({:?}) failed: {}",
            path,
            std::io::Error::last_os_error()
        ));
    }
    Ok(vec![
        sb.f_bsize as NV,
        sb.f_frsize as NV,
        sb.f_blocks as NV,
        sb.f_bfree as NV,
        sb.f_bavail as NV,
        sb.f_files as NV,
        sb.f_ffree as NV,
    ])
}

/// `Mytest2::words($s)` — split a string on whitespace, return the
/// list of substrings. Demonstrates `Vec<String>` return.
#[xs_sub]
fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_string()).collect()
}

xs_boot! {
    package = "Mytest2";
    subs = [foo, shout, byte_len, statfs, words];
}
