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
            let av = crate::thx_call!(perl, Perl_newAV,);
            // `AV` is layout-compatible with `SV` (it starts with the
            // SV header) — `sv_2mortal` accepts a `*mut SV` of the AV.
            crate::thx_call!(perl, Perl_sv_2mortal, av as *mut SV);
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
            crate::thx_call!(perl, Perl_av_push, self.0.as_ptr(), inc);
        }
    }

    /// Wrap this AV in a fresh mortal `RV` (`\@array` in Perl). The
    /// returned `Rv<Av>` is the value you typically push to the Perl
    /// stack as the XS sub's return.
    #[inline]
    pub fn into_rv(self, perl: &Perl) -> Rv<Av> {
        unsafe {
            // `Perl_newRV` is the refcount-incrementing flavor of the
            // C `newRV` macro: it bumps the AV's refcount and yields a
            // fresh RV with refcount 1. Mortalize so it's freed at
            // scope exit too.
            let rv = crate::thx_call!(perl, Perl_newRV, self.0.as_ptr() as *mut SV);
            crate::thx_call!(perl, Perl_sv_2mortal, rv);
            Rv::from_raw_sv(rv)
        }
    }

    /// Wrap a raw `*mut AV` without checking for null. Used by the
    /// `#[xs_sub]` proc-macro after it has dereferenced an `&Av` arg
    /// (caller passed `\@arr` and we've already SvROK / SvTYPE-checked
    /// the SV).
    ///
    /// # Safety
    /// `p` must be non-null and point to a valid AV that outlives the
    /// returned `Av`.
    #[inline]
    pub unsafe fn from_raw_unchecked(p: *mut AV) -> Av {
        debug_assert!(!p.is_null(), "Av::from_raw_unchecked received a null pointer");
        Av(unsafe { NonNull::new_unchecked(p) })
    }

    /// Number of elements (`scalar @array`).
    #[inline]
    pub fn len(&self, perl: &Perl) -> usize {
        // `av_len` returns the highest index, or -1 for empty.
        let n = unsafe { crate::thx_call!(perl, Perl_av_len, self.0.as_ptr()) };
        if n < 0 { 0 } else { (n + 1) as usize }
    }

    /// `$arr[$idx]`, or `None` if the slot is empty / out of bounds.
    /// The returned `Sv` borrows from this AV — don't keep it past
    /// any mutation of the AV.
    #[inline]
    pub fn get(&self, perl: &Perl, idx: usize) -> Option<Sv> {
        let svp = unsafe {
            crate::thx_call!(perl, Perl_av_fetch, self.0.as_ptr(), idx as isize, 0)
        };
        if svp.is_null() {
            return None;
        }
        // av_fetch yields `**SV`; deref to get the slot's `*mut SV`.
        // The slot may itself be null for sparse arrays.
        Sv::from_raw(unsafe { *svp })
    }

    /// Iterate over `(0..len)` yielding each slot as `Option<Sv>`.
    /// `None` for sparse / unallocated slots.
    #[inline]
    pub fn iter<'a>(&'a self, perl: &'a Perl) -> AvIter<'a> {
        let len = self.len(perl);
        AvIter { perl, av: self.0, idx: 0, len }
    }

    /// Raw pointer for FFI.
    #[inline]
    pub fn as_ptr(&self) -> *mut AV {
        self.0.as_ptr()
    }
}

/// Iterator yielded by [`Av::iter`].
pub struct AvIter<'a> {
    perl: &'a Perl,
    av: NonNull<AV>,
    idx: usize,
    len: usize,
}

impl<'a> Iterator for AvIter<'a> {
    type Item = Option<Sv>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            return None;
        }
        let i = self.idx;
        self.idx += 1;
        let svp = unsafe {
            crate::thx_call!(self.perl, Perl_av_fetch, self.av.as_ptr(), i as isize, 0)
        };
        Some(if svp.is_null() {
            None
        } else {
            Sv::from_raw(unsafe { *svp })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let r = self.len - self.idx;
        (r, Some(r))
    }
}
