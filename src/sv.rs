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

use libperl_sys::{AV, CV, HV, PerlInterpreter, SV, svtype};

use crate::{Av, Cv, Gv, Hv, Perl};

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

    // ── Constructors (Phase 3.10c) ─────────────────────────────────
    //
    // Mortal-forced policy: every constructor `Perl_sv_2mortal`s the
    // newly-allocated SV before returning. The caller gets a non-owning
    // handle that is automatically freed at end of expression unless
    // something else (e.g. `av_push`) takes a refcount on it. This
    // matches the XS T_SV typemap convention and means leaks are not
    // possible by mere construction.

    /// Allocate a fresh mortal SV holding `v` as an integer.
    #[inline]
    pub fn new_iv(perl: &Perl, v: crate::IV) -> Sv {
        unsafe {
            let raw = crate::thx_call!(perl, Perl_newSViv, v);
            let raw = crate::thx_call!(perl, Perl_sv_2mortal, raw);
            Sv::from_raw_unchecked(raw)
        }
    }

    /// Allocate a fresh mortal SV holding `v` as an unsigned integer.
    #[inline]
    pub fn new_uv(perl: &Perl, v: crate::UV) -> Sv {
        unsafe {
            let raw = crate::thx_call!(perl, Perl_newSVuv, v);
            let raw = crate::thx_call!(perl, Perl_sv_2mortal, raw);
            Sv::from_raw_unchecked(raw)
        }
    }

    /// Allocate a fresh mortal SV holding `v` as a double.
    #[inline]
    pub fn new_nv(perl: &Perl, v: crate::NV) -> Sv {
        unsafe {
            let raw = crate::thx_call!(perl, Perl_newSVnv, v);
            let raw = crate::thx_call!(perl, Perl_sv_2mortal, raw);
            Sv::from_raw_unchecked(raw)
        }
    }

    // ── Accessors (Phase 3.10d) ────────────────────────────────────
    //
    // `SvIV` / `SvUV` / `SvNV` are macrogen-emitted helpers whose
    // signature differs across threading modes — wrap them in
    // threading-aware methods so user code in a body fn doesn't have
    // to reach for `thx_call!` directly.

    /// `SvIV($sv)` — coerce the SV to an integer.
    #[inline]
    pub fn iv(&self, perl: &Perl) -> crate::IV {
        unsafe { crate::thx_call!(perl, SvIV, self.0.as_ptr()) }
    }

    /// `SvUV($sv)` — coerce to unsigned integer.
    #[inline]
    pub fn uv(&self, perl: &Perl) -> crate::UV {
        unsafe { crate::thx_call!(perl, SvUV, self.0.as_ptr()) }
    }

    /// `SvNV($sv)` — coerce to floating point.
    #[inline]
    pub fn nv(&self, perl: &Perl) -> crate::NV {
        unsafe { crate::thx_call!(perl, SvNV, self.0.as_ptr()) }
    }

    /// `SvPV($sv)` — coerce to a string and return the byte slice
    /// borrowed from the SV's PV buffer. `'a` is tied to `&self`,
    /// so the slice is invalidated by any mutation that could
    /// reallocate the buffer (`sv_setpv`, `sv_grow`, ...).
    ///
    /// Calls `Perl_sv_2pv_flags` with `SV_GMAGIC` so magic /
    /// overload methods fire — the same as Perl's `"$sv"`.
    #[inline]
    pub fn pv<'a>(&'a self, perl: &Perl) -> &'a [u8] {
        let mut len: libperl_sys::STRLEN = 0;
        let ptr = unsafe {
            crate::thx_call!(
                perl,
                Perl_sv_2pv_flags,
                self.0.as_ptr(),
                &mut len,
                // `SV_GMAGIC` is `U32` on modern Perl, `I32` on
                // 5.30/5.32 — `as _` lets rustc infer.
                libperl_sys::SV_GMAGIC as _,
            )
        };
        if ptr.is_null() {
            return &[];
        }
        unsafe { ::core::slice::from_raw_parts(ptr as *const u8, len as usize) }
    }

    /// Classify this SV as a [`SvKind`] — the rebuilt version of
    /// proto0's `Sv` extraction enum (Step 2 in `docs/plan/README.md`).
    /// Pure struct/flag reads via macrogen accessors; no magic is
    /// triggered and no interpreter context is needed.
    pub fn kind(&self) -> SvKind {
        let sv = self.as_ptr();
        unsafe {
            // GP-carrying globs first: `SVt_PVLV` can be a glob too,
            // so the flag test must precede the type match.
            if libperl_sys::isGV_with_GP(sv) {
                return SvKind::Glob(Gv::from_raw_unchecked(sv as *mut libperl_sys::GV));
            }
            match libperl_sys::SvTYPE(sv) {
                svtype::SVt_PVAV => SvKind::Array(Av::from_raw_unchecked(sv as *mut AV)),
                svtype::SVt_PVHV => SvKind::Hash(Hv::from_raw_unchecked(sv as *mut HV)),
                svtype::SVt_PVCV => SvKind::Code(Cv::from_raw_unchecked(sv as *mut CV)),
                svtype::SVt_REGEXP => SvKind::Regexp(*self),
                t if (t as u32) < (svtype::SVt_PVAV as u32) => {
                    if libperl_sys::SvROK(sv) != 0 {
                        // A reference; the referent is never null.
                        SvKind::Ref(Sv::from_raw_unchecked(libperl_sys::SvRV(sv)))
                    } else {
                        SvKind::Scalar(*self)
                    }
                }
                t => SvKind::Other(t, *self),
            }
        }
    }

    /// Allocate a fresh mortal SV holding `s` as a UTF-8 string.
    pub fn new_pv(perl: &Perl, s: &str) -> Sv {
        let bytes = s.as_bytes();
        unsafe {
            let raw = crate::thx_call!(
                perl,
                Perl_newSVpvn,
                bytes.as_ptr() as *const ::core::ffi::c_char,
                bytes.len() as _,
            );
            // Mark UTF-8: use the same `i64` width-bridge trick as the
            // proc-macro's String push code (sv_flags is U32 on modern
            // Perl, I32 on 5.30 / 5.32).
            let cur: i64 = (*raw).sv_flags as i64;
            (*raw).sv_flags = (cur | (libperl_sys::SVf_UTF8 as i64)) as _;
            let raw = crate::thx_call!(perl, Perl_sv_2mortal, raw);
            Sv::from_raw_unchecked(raw)
        }
    }
}

// `*mut SV` is not Send/Sync; the `NonNull` wrapper inherits this.
// No `unsafe impl Send/Sync for Sv` here — interpreter affinity is
// preserved.

/// Classification of an SV, yielded by [`Sv::kind`]. Payloads are the
/// matching newtype handles (same pointer, viewed through the right
/// type) — non-owning, like the newtypes themselves.
#[derive(Clone, Copy)]
pub enum SvKind {
    /// Plain scalar (IV / NV / PV / undef — anything below `SVt_PVAV`
    /// that is not a reference).
    Scalar(Sv),
    /// A reference; the payload is the *referent* (call `kind()` on
    /// it again to see what it points at).
    Ref(Sv),
    Array(Av),
    Hash(Hv),
    Code(Cv),
    /// GP-carrying glob (symbol-table entry).
    Glob(Gv),
    Regexp(Sv),
    /// Recognised but not further classified (`SVt_PVIO`, ...).
    Other(svtype, Sv),
}

impl std::fmt::Debug for SvKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Pointers carry no printable identity; show the variant, and
        // for `Other` the svtype.
        match self {
            SvKind::Scalar(_) => f.write_str("SvKind::Scalar"),
            SvKind::Ref(_) => f.write_str("SvKind::Ref"),
            SvKind::Array(_) => f.write_str("SvKind::Array"),
            SvKind::Hash(_) => f.write_str("SvKind::Hash"),
            SvKind::Code(_) => f.write_str("SvKind::Code"),
            SvKind::Glob(_) => f.write_str("SvKind::Glob"),
            SvKind::Regexp(_) => f.write_str("SvKind::Regexp"),
            SvKind::Other(t, _) => write!(f, "SvKind::Other({t:?})"),
        }
    }
}

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
#[cfg(all(not(perl_useithreads), perlapi_ver28))]
#[inline]
pub fn sv_undef_ptr(_my_perl: *mut PerlInterpreter) -> *mut SV {
    // In non-threaded Perl 5.28+, `PL_sv_undef` is `#define`d to
    // `PL_sv_immortals[1]` in `perl.h`, and bindgen does not emit
    // a `PL_sv_undef` static. The `PL_sv_immortals` array does
    // exist as a global static, so we index into it directly.
    unsafe { &raw mut libperl_sys::PL_sv_immortals[1] as *mut SV }
}

/// Pointer to the immortal `PL_sv_undef` (non-threaded build,
/// perl < 5.28).
#[cfg(all(not(perl_useithreads), not(perlapi_ver28)))]
#[inline]
pub fn sv_undef_ptr(_my_perl: *mut PerlInterpreter) -> *mut SV {
    // Before the 5.28 immortals-array rework, non-threaded
    // `PL_sv_undef` is a plain global `SV`, emitted by bindgen as a
    // static of its own. Taking a `&raw mut` of a `static mut` needs
    // no unsafe block (unlike the array indexing above).
    &raw mut libperl_sys::PL_sv_undef as *mut SV
}
