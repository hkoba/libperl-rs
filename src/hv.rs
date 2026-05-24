//! `Hv` newtype — `NonNull<HV>` wrapper. Same shape as [`Av`](crate::Av)
//! but for Perl hashes. Mortal-forced construction.

use std::ptr::NonNull;

use libperl_sys::{HV, SV};

use crate::{Perl, Rv, Sv, sv_refcnt_inc};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Hv(NonNull<HV>);

impl Hv {
    /// Allocate a fresh, empty mortal `HV`.
    #[inline]
    pub fn new(perl: &Perl) -> Hv {
        unsafe {
            let hv = crate::thx_call!(perl, Perl_newHV,);
            crate::thx_call!(perl, Perl_sv_2mortal, hv as *mut SV);
            Hv(NonNull::new(hv).expect("Perl_newHV returned null"))
        }
    }

    /// `$hv{$key} = $val` — store `val` under `key`. Like
    /// [`Av::push`](crate::Av::push), the value is refcount-inc'd
    /// before being handed off because `hv_store` takes ownership of
    /// one ref.
    pub fn store(&self, perl: &Perl, key: &str, val: Sv) {
        let bytes = key.as_bytes();
        unsafe {
            let inc = sv_refcnt_inc(val.as_ptr());
            // `klen` is `I32` — caller-side cast; len always fits for
            // realistic hash keys, no overflow check.
            crate::thx_call!(
                perl,
                Perl_hv_store,
                self.0.as_ptr(),
                bytes.as_ptr() as *const ::core::ffi::c_char,
                bytes.len() as _,
                inc,
                0, // hash = 0 → let perl compute
            );
        }
    }

    /// Wrap this HV in a fresh mortal `RV` (`\%hash` in Perl).
    #[inline]
    pub fn into_rv(self, perl: &Perl) -> Rv<Hv> {
        unsafe {
            let rv = crate::thx_call!(perl, Perl_newRV, self.0.as_ptr() as *mut SV);
            crate::thx_call!(perl, Perl_sv_2mortal, rv);
            Rv::from_raw_sv(rv)
        }
    }

    /// Wrap a raw `*mut HV` without checking for null. Used by the
    /// `#[xs_sub]` proc-macro after it has dereferenced an `&Hv` arg.
    ///
    /// # Safety
    /// `p` must be non-null and point to a valid HV that outlives the
    /// returned `Hv`.
    #[inline]
    pub unsafe fn from_raw_unchecked(p: *mut HV) -> Hv {
        debug_assert!(!p.is_null(), "Hv::from_raw_unchecked received a null pointer");
        Hv(unsafe { NonNull::new_unchecked(p) })
    }

    /// Iterate over `(key, value)` pairs. Resets the hash's iterator
    /// state on entry, so a single live `HvIter` is fine; nested or
    /// concurrent iteration over the same HV will interfere.
    ///
    /// Keys come back as `&[u8]` (raw bytes) — Perl hash keys aren't
    /// required to be UTF-8. Borrows last as long as the iterator;
    /// don't mutate the HV while iterating.
    #[inline]
    pub fn iter<'a>(&'a self, perl: &'a Perl) -> HvIter<'a> {
        unsafe { crate::thx_call!(perl, Perl_hv_iterinit, self.0.as_ptr()); }
        HvIter { perl, hv: self.0, _marker: ::core::marker::PhantomData }
    }

    /// Raw pointer for FFI.
    #[inline]
    pub fn as_ptr(&self) -> *mut HV {
        self.0.as_ptr()
    }
}

/// Iterator yielded by [`Hv::iter`].
pub struct HvIter<'a> {
    perl: &'a Perl,
    hv: NonNull<HV>,
    // Conceptually borrows the HV's bucket storage for the key slice.
    _marker: ::core::marker::PhantomData<&'a HV>,
}

impl<'a> Iterator for HvIter<'a> {
    type Item = (&'a [u8], Sv);

    fn next(&mut self) -> Option<Self::Item> {
        let mut key_ptr: *mut ::core::ffi::c_char = ::core::ptr::null_mut();
        let mut keylen: libperl_sys::I32 = 0;
        let val = unsafe {
            crate::thx_call!(
                self.perl,
                Perl_hv_iternextsv,
                self.hv.as_ptr(),
                &mut key_ptr,
                &mut keylen,
            )
        };
        if val.is_null() {
            return None;
        }
        let key = unsafe {
            ::core::slice::from_raw_parts(key_ptr as *const u8, keylen as usize)
        };
        let sv = unsafe { Sv::from_raw_unchecked(val) };
        Some((key, sv))
    }
}
