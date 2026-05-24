//! Mytest2 — demo of `#[xs_sub]` Phase 3.8 features:
//! `&CStr` / `&str` arguments and `String` / `NV` return values
//! (perlxstut EXAMPLE 4 territory).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::CStr;

use libperl_rs::{xs_boot, xs_sub, Av, Hv, IV, NV, Perl, Rv, SV, Sv, UV};

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

/// `Mytest2::identity($sv)` — return the input SV unchanged.
/// Demonstrates `*mut SV` raw passthrough on both arg and return.
#[xs_sub]
fn identity(sv: *mut SV) -> *mut SV {
    sv
}

/// `Mytest2::maybe_sv($sv, $keep)` — if `$keep` is true, return `$sv`,
/// else return `undef`. Demonstrates `Option<*mut SV>` mapping to
/// `XSRETURN_UNDEF` on `None`.
#[xs_sub]
fn maybe_sv(sv: *mut SV, keep: IV) -> Option<*mut SV> {
    if keep != 0 { Some(sv) } else { None }
}

/// `Mytest2::identity_sv($sv)` — same as `identity` but using the
/// `Sv` newtype on both sides (Phase 3.10b).
#[xs_sub]
fn identity_sv(sv: Sv) -> Sv {
    sv
}

/// `Mytest2::maybe_sv2($sv, $keep)` — `Option<Sv>` analogue of
/// `maybe_sv` (Phase 3.10b).
#[xs_sub]
fn maybe_sv2(sv: Sv, keep: IV) -> Option<Sv> {
    if keep != 0 { Some(sv) } else { None }
}

/// `Mytest2::wrap_iv($n)` — Phase 3.10c: take an IV, return a fresh
/// mortal SV holding that integer. Demonstrates `&Perl` context arg
/// + `Sv::new_iv` constructor.
#[xs_sub]
fn wrap_iv(my_perl: &Perl, n: IV) -> Sv {
    Sv::new_iv(my_perl, n)
}

/// `Mytest2::wrap_uv($n)` / `wrap_nv($x)` / `wrap_pv($s)` — analogue
/// constructors covering the rest of the SV scalar flavors.
#[xs_sub]
fn wrap_uv(my_perl: &Perl, n: UV) -> Sv {
    Sv::new_uv(my_perl, n)
}

#[xs_sub]
fn wrap_nv(my_perl: &Perl, x: NV) -> Sv {
    Sv::new_nv(my_perl, x)
}

#[xs_sub]
fn wrap_pv(my_perl: &Perl, s: &str) -> Sv {
    Sv::new_pv(my_perl, s)
}

/// `Mytest2::make_pair()` — return `[1, 2]` as an array reference.
/// Demonstrates `Av::new` / `Av::push` / `Av::into_rv` and the
/// `Rv<Av>` return path.
#[xs_sub]
fn make_pair(my_perl: &Perl) -> Rv<Av> {
    let av = Av::new(my_perl);
    av.push(my_perl, Sv::new_iv(my_perl, 1));
    av.push(my_perl, Sv::new_iv(my_perl, 2));
    av.into_rv(my_perl)
}

/// `Mytest2::make_record()` — return `{ name => "ada", year => 1815 }`
/// as a hash reference. Demonstrates `Hv::new` / `Hv::store` /
/// `Hv::into_rv` and the `Rv<Hv>` return path.
#[xs_sub]
fn make_record(my_perl: &Perl) -> Rv<Hv> {
    let hv = Hv::new(my_perl);
    hv.store(my_perl, "name", Sv::new_pv(my_perl, "ada"));
    hv.store(my_perl, "year", Sv::new_iv(my_perl, 1815));
    hv.into_rv(my_perl)
}

/// `Mytest2::maybe_pair($keep)` — `Some(\@arr)` or `None` →
/// undef. Exercises `Option<Rv<Av>>`.
#[xs_sub]
fn maybe_pair(my_perl: &Perl, keep: IV) -> Option<Rv<Av>> {
    if keep != 0 {
        let av = Av::new(my_perl);
        av.push(my_perl, Sv::new_iv(my_perl, 10));
        av.push(my_perl, Sv::new_iv(my_perl, 20));
        Some(av.into_rv(my_perl))
    } else {
        None
    }
}

/// `Mytest2::sum_iv(\@arr)` — Phase 3.10d: take an array ref, sum
/// its elements as IVs. Demonstrates `&Av` arg + `Av::iter`. Croaks
/// at the trampoline level if the caller passes a non-array-ref.
#[xs_sub]
fn sum_iv(my_perl: &Perl, av: &Av) -> IV {
    let mut acc: IV = 0;
    for slot in av.iter(my_perl) {
        if let Some(sv) = slot {
            acc += sv.iv(my_perl);
        }
    }
    acc
}

/// `Mytest2::av_len_demo(\@arr)` — return the length of the array.
/// Trivial, but exercises the `&Av` + `Av::len` path.
#[xs_sub]
fn av_len_demo(my_perl: &Perl, av: &Av) -> IV {
    av.len(my_perl) as IV
}

/// `Mytest2::record_keys(\%hash)` — Phase 3.10d: take a hash ref,
/// return a sorted Vec of its keys. Demonstrates `&Hv` arg +
/// `Hv::iter`.
#[xs_sub]
fn record_keys(my_perl: &Perl, hv: &Hv) -> Vec<String> {
    let mut keys: Vec<String> = hv
        .iter(my_perl)
        .map(|(k, _v)| String::from_utf8_lossy(k).into_owned())
        .collect();
    keys.sort();
    keys
}

/// `Mytest2::multi_statfs(\@paths)` — perlxstut EXAMPLE 6 territory.
///
/// For each path in the input arrayref, run `statvfs(2)`. Returns a
/// hashref keyed by path. Each value is either a 7-element arrayref
/// (the `statvfs` results, same shape as the existing `statfs` sub)
/// on success, or a string error message on failure. Paths that
/// aren't valid byte strings (interior NULs) are skipped silently.
///
/// End-to-end demo for Phase 3.10e: combines `&Av` input,
/// `Av::iter` + `Sv::pv` to read paths, `Av`/`Hv` construction,
/// `Sv::new_*` for value SVs, `Hv::store` with both `Rv<Av>` (via
/// `into_rv().as_sv()`) and string values, and `Rv<Hv>` return.
#[xs_sub]
fn multi_statfs(my_perl: &Perl, paths: &Av) -> Rv<Hv> {
    let result = Hv::new(my_perl);
    for slot in paths.iter(my_perl) {
        let Some(path_sv) = slot else { continue };
        let path_bytes = path_sv.pv(my_perl);
        let Ok(path_cstring) = std::ffi::CString::new(path_bytes) else {
            continue;
        };
        // SAFETY: `Hv::store` requires `&str` keys. Paths from Perl
        // may not be valid UTF-8; we replace invalid sequences so
        // every path gets a slot (consistent with how Perl hashes
        // would treat the original byte string when stringified).
        let key = String::from_utf8_lossy(path_bytes);

        let mut sb: libc::statvfs = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::statvfs(path_cstring.as_ptr(), &mut sb) };
        if rc == 0 {
            let av = Av::new(my_perl);
            av.push(my_perl, Sv::new_nv(my_perl, sb.f_bsize  as NV));
            av.push(my_perl, Sv::new_nv(my_perl, sb.f_frsize as NV));
            av.push(my_perl, Sv::new_nv(my_perl, sb.f_blocks as NV));
            av.push(my_perl, Sv::new_nv(my_perl, sb.f_bfree  as NV));
            av.push(my_perl, Sv::new_nv(my_perl, sb.f_bavail as NV));
            av.push(my_perl, Sv::new_nv(my_perl, sb.f_files  as NV));
            av.push(my_perl, Sv::new_nv(my_perl, sb.f_ffree  as NV));
            result.store(my_perl, &key, av.into_rv(my_perl).as_sv());
        } else {
            let msg = format!(
                "statvfs failed: {}",
                std::io::Error::last_os_error()
            );
            result.store(my_perl, &key, Sv::new_pv(my_perl, &msg));
        }
    }
    result.into_rv(my_perl)
}

xs_boot! {
    package = "Mytest2";
    subs = [foo, shout, byte_len, statfs, words, identity, maybe_sv,
            identity_sv, maybe_sv2,
            wrap_iv, wrap_uv, wrap_nv, wrap_pv,
            make_pair, make_record, maybe_pair,
            sum_iv, av_len_demo, record_keys,
            multi_statfs];
}
