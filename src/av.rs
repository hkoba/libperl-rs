//! `Av` newtype — `NonNull<AV>` wrapper. Same shape as [`Sv`](crate::Sv)
//! but for Perl arrays. Mortal-forced construction (see §3.10c in
//! `docs/plan/README.md`) means the AV is automatically freed at end
//! of expression unless something else (typically a wrapping `Rv<Av>`)
//! takes a refcount.

use std::ptr::NonNull;

use libperl_sys::{AV, SV};

use crate::{Perl, Rv, Sv, sv_refcnt_inc};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Av(NonNull<AV>);

impl Av {
    /// Allocate a fresh, empty mortal `AV`.
    #[inline]
    pub fn new(perl: &Perl) -> Av {
        unsafe {
            let av = libperl_sys::Perl_newAV(perl.as_ptr());
            // `AV` is layout-compatible with `SV` (it starts with the
            // SV header) — `sv_2mortal` accepts a `*mut SV` of the AV.
            libperl_sys::Perl_sv_2mortal(perl.as_ptr(), av as *mut SV);
            Av(NonNull::new(av).expect("Perl_newAV returned null"))
        }
    }

    /// Append `sv` to the end of the array. The `Sv` is refcount-inc'd
    /// before being handed to `av_push` because `av_push` takes
    /// ownership of one ref — and the caller's mortal `Sv` would
    /// otherwise be freed at scope exit, leaving a dangling slot.
    #[inline]
    pub fn push(&self, perl: &Perl, sv: Sv) {
        unsafe {
            let inc = sv_refcnt_inc(sv.as_ptr());
            libperl_sys::Perl_av_push(perl.as_ptr(), self.0.as_ptr(), inc);
        }
    }

    /// Wrap this AV in a fresh mortal `RV` (`\@array` in Perl). The
    /// returned `Rv<Av>` is the value you typically push to the Perl
    /// stack as the XS sub's return.
    #[inline]
    pub fn into_rv(self, perl: &Perl) -> Rv<Av> {
        unsafe {
            // `newRV` (the macrogen-emitted helper for the C `newRV`
            // macro) is the refcount-incrementing flavor: it bumps the
            // AV's refcount and yields a fresh RV with refcount 1.
            // Mortalize so it's freed at scope exit too.
            let rv = libperl_sys::newRV(perl.as_ptr(), self.0.as_ptr() as *mut SV);
            libperl_sys::Perl_sv_2mortal(perl.as_ptr(), rv);
            Rv::from_raw_sv(rv)
        }
    }

    /// Raw pointer for FFI.
    #[inline]
    pub fn as_ptr(&self) -> *mut AV {
        self.0.as_ptr()
    }
}
