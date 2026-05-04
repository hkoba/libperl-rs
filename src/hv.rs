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
            let hv = libperl_sys::Perl_newHV(perl.as_ptr());
            libperl_sys::Perl_sv_2mortal(perl.as_ptr(), hv as *mut SV);
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
            libperl_sys::Perl_hv_store(
                perl.as_ptr(),
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
            let rv = libperl_sys::newRV(perl.as_ptr(), self.0.as_ptr() as *mut SV);
            libperl_sys::Perl_sv_2mortal(perl.as_ptr(), rv);
            Rv::from_raw_sv(rv)
        }
    }

    /// Raw pointer for FFI.
    #[inline]
    pub fn as_ptr(&self) -> *mut HV {
        self.0.as_ptr()
    }
}
