//! Small portable helpers for SV-pointer manipulation that the
//! `#[xs_sub]` proc-macro relies on. Living here (rather than in the
//! generated proc-macro output) lets us bridge per-Perl-version /
//! per-build-mode differences in one place.
//!
//! Currently provides:
//!
//!   * [`sv_refcnt_inc`] — `SvREFCNT_inc` equivalent. The
//!     macrogen-emitted `Perl_SvREFCNT_inc` is not generated on
//!     older Perls (5.30 / 5.32 / 5.34), so we inline the increment
//!     directly.
//!   * [`sv_undef_ptr`] — pointer to the immortal `PL_sv_undef`. In
//!     threaded Perl this is the per-interpreter `Isv_undef` field;
//!     in non-threaded Perl it lives in the global `PL_sv_immortals`
//!     array (`PL_sv_undef` is itself `#define`d to `PL_sv_immortals[1]`
//!     in `perl.h`, so bindgen does not emit it as its own static).

use libperl_sys::{PerlInterpreter, SV};

/// `SvREFCNT_inc(sv)` — bump the refcount of `sv` by one and return
/// it (or just the null pointer on null input).
///
/// `unsafe` because the caller must guarantee `sv` is either null or
/// points to a valid SV.
#[inline]
pub unsafe fn sv_refcnt_inc(sv: *mut SV) -> *mut SV {
    if !sv.is_null() {
        // `sv_refcnt` is `U32` on modern Perl and `I32` on older
        // ones; `+= 1` with an integer literal type-infers to either.
        // Saturating wraparound is fine — refcounts in practice
        // never approach `U32::MAX`.
        unsafe { (*sv).sv_refcnt = (*sv).sv_refcnt.wrapping_add(1); }
    }
    sv
}

/// Pointer to the immortal `PL_sv_undef`. Use as the return slot of
/// an XSUB to mean "return undef" (`XSRETURN_UNDEF` equivalent).
#[cfg(perl_useithreads)]
#[inline]
pub fn sv_undef_ptr(my_perl: *mut PerlInterpreter) -> *mut SV {
    // In threaded Perl, `PL_sv_undef` is the per-interpreter
    // `Isv_undef` field. `&raw mut` avoids creating a `&mut` borrow.
    unsafe { &raw mut (*my_perl).Isv_undef as *mut SV }
}

/// Pointer to the immortal `PL_sv_undef` (non-threaded build).
#[cfg(not(perl_useithreads))]
#[inline]
pub fn sv_undef_ptr(_my_perl: *mut PerlInterpreter) -> *mut SV {
    // In non-threaded Perl, `PL_sv_undef` is `#define`d to
    // `PL_sv_immortals[1]` in `perl.h`, and bindgen does not emit
    // a `PL_sv_undef` static. The `PL_sv_immortals` array does
    // exist as a global static, so we index into it directly.
    unsafe { &raw mut libperl_sys::PL_sv_immortals[1] as *mut SV }
}
