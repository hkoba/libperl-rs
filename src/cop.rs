//! `Cop` newtype — a non-null handle to a Perl `COP` ("control op":
//! the `nextstate` / `dbstate` statement-boundary ops that carry
//! source location). This is the source↔OP mapping piece of Step 2 —
//! `CopFILE` / `CopLINE` extraction per `docs/plan/README.md` §4
//! Step 3.3, promoted into the newtype layer.
//!
//! Obtain one via [`Op::as_cop`](crate::Op::as_cop) (class-checked) or
//! [`Cv::first_cop`](crate::Cv::first_cop).

use std::ptr::NonNull;

use libperl_sys::COP;

/// Non-null pointer to a Perl `COP`. Same ABI as `*mut COP`.
/// Non-owning, like the other newtypes.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Cop(NonNull<COP>);

impl Cop {
    /// Wrap a raw COP pointer without checking for null.
    ///
    /// # Safety
    /// Caller must guarantee `p` is non-null and points to a valid
    /// COP for at least the lifetime of the resulting `Cop`. Prefer
    /// [`Op::as_cop`](crate::Op::as_cop), which class-checks first.
    #[inline]
    pub unsafe fn from_raw_unchecked(p: *const COP) -> Self {
        debug_assert!(!p.is_null(), "Cop::from_raw_unchecked received a null pointer");
        Cop(unsafe { NonNull::new_unchecked(p as *mut COP) })
    }

    /// Wrap a raw COP pointer, returning `None` on null input.
    #[inline]
    pub fn from_raw(p: *const COP) -> Option<Self> {
        NonNull::new(p as *mut COP).map(Cop)
    }

    /// Raw pointer for FFI calls.
    #[inline]
    pub fn as_ptr(&self) -> *mut COP {
        self.0.as_ptr()
    }

    /// Source line of the statement this COP opens (`CopLINE`).
    #[inline]
    pub fn line(&self) -> u32 {
        unsafe { libperl_sys::CopLINE(self.0.as_ptr()) }
    }

    /// Source file of the statement (`CopFILE`); `"-e"` for one-liner
    /// scripts, `"(eval N)"` inside string evals.
    pub fn file(&self) -> Option<String> {
        let p = unsafe { libperl_sys::CopFILE(self.0.as_ptr()) };
        if p.is_null() {
            None
        } else {
            Some(
                unsafe { std::ffi::CStr::from_ptr(p) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
}
