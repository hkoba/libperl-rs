//! `Rv<T>` — typed reference SV (`\$scalar`, `\@array`, `\%hash`).
//!
//! `Rv<T>` is at the C level just an `SV` whose body holds a pointer
//! to another SV / AV / HV. The `T` type parameter is a Rust-side hint
//! for what's at the other end — it makes `Av::into_rv` return
//! `Rv<Av>` instead of an opaque `Rv`, so the caller's intent shows up
//! in signatures.
//!
//! `Rv<T>` is constructed via [`Av::into_rv`](crate::Av::into_rv) /
//! [`Hv::into_rv`](crate::Hv::into_rv); both produce a fresh mortal
//! RV so the proc-macro can push it directly without further refcount
//! bookkeeping.

use std::marker::PhantomData;

use libperl_sys::SV;

use crate::Sv;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Rv<T> {
    sv: Sv,
    // `*const T` instead of `T` so `Rv<T>` is `Copy` regardless of T.
    _phantom: PhantomData<*const T>,
}

impl<T> Rv<T> {
    /// Wrap a raw `*mut SV` known to be an RV pointing to a `T`.
    ///
    /// # Safety
    /// Caller asserts `raw` is a non-null SV with `SVt_RV` body.
    #[inline]
    pub unsafe fn from_raw_sv(raw: *mut SV) -> Rv<T> {
        Rv {
            sv: unsafe { Sv::from_raw_unchecked(raw) },
            _phantom: PhantomData,
        }
    }

    /// The underlying RV as an `Sv`.
    #[inline]
    pub fn as_sv(&self) -> Sv {
        self.sv
    }

    /// Raw `*mut SV` for FFI / proc-macro stack push.
    #[inline]
    pub fn as_ptr(&self) -> *mut SV {
        self.sv.as_ptr()
    }
}
