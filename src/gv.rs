//! `Gv` newtype — a non-null handle to a Perl `GV` (glob value), the
//! symbol-table entry type. Step 2's name-resolution piece: from a
//! stash slot or a CV to package-qualified names and to the CV / file
//! / line behind the glob.
//!
//! All accessors delegate to the macrogen-emitted official API
//! (`isGV_with_GP`, `GvNAME_HEK`, `GvSTASH`, `HEK_KEY`, `HvNAME`,
//! `GvGP`, `GvCV`, `GvFILE`, `GvLINE` — generation verified across
//! 5.28-5.44, both threading modes). The generated GP-slot macros
//! dereference `GvGP` without a null check, faithful to the C
//! macros, so the methods here add the null guard and Option-ify
//! the result.

use std::ptr::NonNull;

use libperl_sys::{GP, GV, HEK, SV};

use crate::Cv;

/// Non-null pointer to a Perl `GV`. Same ABI as `*mut GV`.
/// Non-owning, like the other newtypes.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Gv(NonNull<GV>);

impl Gv {
    /// Wrap a raw GV pointer without checking for null.
    ///
    /// # Safety
    /// Caller must guarantee `p` is non-null and points to a valid,
    /// GP-carrying GV for at least the lifetime of the resulting
    /// `Gv`. Prefer [`Gv::from_sv`], which checks both.
    #[inline]
    pub unsafe fn from_raw_unchecked(p: *mut GV) -> Self {
        debug_assert!(!p.is_null(), "Gv::from_raw_unchecked received a null pointer");
        Gv(unsafe { NonNull::new_unchecked(p) })
    }

    /// Wrap a raw GV pointer, returning `None` on null input. The
    /// pointer must already be known to be a real GV (e.g. the return
    /// of `CvGV`); for an arbitrary SV use [`Gv::from_sv`].
    #[inline]
    pub fn from_raw(p: *mut GV) -> Option<Self> {
        NonNull::new(p).map(Gv)
    }

    /// View an arbitrary SV as a GV when it is one. Uses the official
    /// `isGV_with_GP` test, so `SVt_PVLV`-shaped globs are accepted
    /// and non-glob values (including GP-less GV shells) are not.
    #[inline]
    pub fn from_sv(sv: *mut SV) -> Option<Gv> {
        if sv.is_null() || !unsafe { libperl_sys::isGV_with_GP(sv) } {
            return None;
        }
        Some(unsafe { Gv::from_raw_unchecked(sv as *mut GV) })
    }

    /// Raw pointer for FFI calls.
    #[inline]
    pub fn as_ptr(&self) -> *mut GV {
        self.0.as_ptr()
    }

    /// The glob's own name (`GvNAME_HEK` → `HEK_KEY`), without the
    /// package part: `"foo"` for `*Foo::foo`.
    pub fn name(&self) -> Option<String> {
        let hek = unsafe { libperl_sys::GvNAME_HEK(self.as_ptr() as *const SV) };
        hek_str(hek)
    }

    /// Name of the stash the glob lives in (`GvSTASH` → `HvNAME`):
    /// `"Foo"` for `*Foo::foo`.
    pub fn stash_name(&self) -> Option<String> {
        let stash = unsafe { libperl_sys::GvSTASH(self.as_ptr() as *const SV) };
        if stash.is_null() {
            return None;
        }
        cstr_opt(unsafe { libperl_sys::HvNAME(stash) })
    }

    /// Package-qualified name: `"Foo::foo"`, or just `"foo"` when the
    /// stash is unnamed. `None` when the glob has no name at all.
    pub fn qualified_name(&self) -> Option<String> {
        let name = self.name()?;
        Some(match self.stash_name() {
            Some(pkg) => format!("{pkg}::{name}"),
            None => name,
        })
    }

    /// The glob's GP ("glob pointer" — the shared slot block), or
    /// null for GP-less GV shells (`GvGP`).
    #[inline]
    fn gp(&self) -> *mut GP {
        // Inferred casts here and below: the generated Gv accessors
        // take `*const SV` except `GvCV` (`*const GV`); `as *const _`
        // fits each (same lesson as `Cv::gv`'s 5.32 quirk).
        unsafe { libperl_sys::GvGP(self.as_ptr() as *const _) }
    }

    /// The CV in the glob's CODE slot (`GvCV`), if any.
    #[inline]
    pub fn cv(&self) -> Option<Cv> {
        if self.gp().is_null() {
            return None;
        }
        Cv::from_raw(unsafe { libperl_sys::GvCV(self.as_ptr() as *const _) })
    }

    /// Source file where the glob was first created (`GvFILE`).
    pub fn file(&self) -> Option<String> {
        if self.gp().is_null() {
            return None;
        }
        cstr_opt(unsafe { libperl_sys::GvFILE(self.as_ptr() as *const _) })
    }

    /// Source line where the glob was first created (`GvLINE`).
    /// `None` when the glob has no GP.
    pub fn line(&self) -> Option<u32> {
        if self.gp().is_null() {
            return None;
        }
        Some(unsafe { libperl_sys::GvLINE(self.as_ptr() as *const _) })
    }
}

fn cstr_opt(p: *const std::os::raw::c_char) -> Option<String> {
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

fn hek_str(hek: *const HEK) -> Option<String> {
    if hek.is_null() {
        None
    } else {
        cstr_opt(unsafe { libperl_sys::HEK_KEY(hek) })
    }
}
