//! `Sv` newtype + small portable helpers for SV-pointer manipulation
//! that the `#[xs_sub]` proc-macro relies on. Living here (rather
//! than in the generated proc-macro output) lets us bridge
//! per-Perl-version / per-build-mode differences in one place.
//!
//! See `docs/plan/README.md` §3.10 for the layered roll-out:
//!
//!   * Phase 3.10a (raw `*mut SV` passthrough): [`sv_refcnt_inc`] +
//!     [`sv_undef_ptr`] helpers below.
//!   * Phase 3.10b (this module): the [`Sv`] newtype itself —
//!     `NonNull<SV>` wrapper that the proc-macro recognises as both
//!     argument and return type. Constructors (`Sv::new_iv`, etc.)
//!     and getter methods land in 3.10c when the demos for
//!     `Av` / `Hv` need them and we have a clear story for "where
//!     does `&Perl` come from inside the body."

use std::ptr::NonNull;

use libperl_sys::{PerlInterpreter, SV};

/// A non-null pointer to a Perl `SV`. Same ABI as `*mut SV` — `NonNull`
/// is a `#[repr(transparent)]` wrapper that just encodes the
/// non-null invariant in the type. See `docs/plan/README.md` §3.4
/// for why we use `NonNull` instead of `&mut SV`.
///
/// `Sv` does **not** own its referent: dropping an `Sv` is a no-op,
/// which matches Perl's refcount-based ownership model. Construction
/// methods (Phase 3.10c) will mortalise newly-created SVs so they
/// survive the call but get freed at end of expression — caller
/// doesn't need to worry about leaks.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Sv(NonNull<SV>);

impl Sv {
    /// Wrap a raw `*mut SV` without checking for null.
    ///
    /// # Safety
    /// Caller must guarantee `p` is non-null and points to a valid SV
    /// for at least the lifetime of the resulting `Sv`. The proc-macro
    /// uses this on the `*mut SV` it pulled off the Perl stack — those
    /// pointers are always non-null.
    #[inline]
    pub unsafe fn from_raw_unchecked(p: *mut SV) -> Self {
        debug_assert!(!p.is_null(), "Sv::from_raw_unchecked received a null pointer");
        Sv(unsafe { NonNull::new_unchecked(p) })
    }

    /// Wrap a raw `*mut SV`, returning `None` on null input. Use this
    /// when the SV pointer comes from a fallible source (e.g.
    /// `av_fetch`).
    #[inline]
    pub fn from_raw(p: *mut SV) -> Option<Self> {
        NonNull::new(p).map(Sv)
    }

    /// Raw `*mut SV` for FFI calls. Same address as the underlying
    /// SV — no allocation, no copy.
    #[inline]
    pub fn as_ptr(&self) -> *mut SV {
        self.0.as_ptr()
    }
}

// `*mut SV` is not Send/Sync; the `NonNull` wrapper inherits this.
// No `unsafe impl Send/Sync for Sv` here — interpreter affinity is
// preserved.

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
