//! `Gv` newtype — a non-null handle to a Perl `GV` (glob value), the
//! symbol-table entry type. Step 2's name-resolution piece: from a
//! stash slot or a CV to package-qualified names and to the CV / file
//! / line behind the glob.
//!
//! Name accessors delegate to macrogen-emitted official API
//! (`GvNAME_HEK`, `GvSTASH`, `HEK_KEY`, `HvNAME`, `isGV_with_GP`).
//! The GP-slot accessors (`cv` / `file` / `line`) read the `gp`
//! struct directly instead: the corresponding macros are not
//! generatable across the whole support range (`GvGP` — and with it
//! `GvCV` / `GvFILE` / `GvLINE` — is `CASCADE_UNAVAILABLE` on perl
//! 5.44), while the struct layout is stable. Same approach as
//! perl-LibPerlRs-PartialEval's `raw.rs::gv_cv`.

use std::ptr::NonNull;

use libperl_sys::{GV, HEK, SV, gp};

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
    /// null. Direct struct read; see the module doc for why this
    /// doesn't go through a generated `GvGP`.
    #[inline]
    fn gp(&self) -> *mut gp {
        unsafe { (*self.0.as_ptr()).sv_u.svu_gp }
    }

    /// The CV in the glob's CODE slot (`GvCV` equivalent), if any.
    #[inline]
    pub fn cv(&self) -> Option<Cv> {
        let gp = self.gp();
        if gp.is_null() {
            return None;
        }
        Cv::from_raw(unsafe { (*gp).gp_cv })
    }

    /// Source file where the glob was first created (`GvFILE`
    /// equivalent, from `gp_file_hek`).
    pub fn file(&self) -> Option<String> {
        let gp = self.gp();
        if gp.is_null() {
            return None;
        }
        hek_str(unsafe { (*gp).gp_file_hek })
    }

    /// Source line where the glob was first created (`GvLINE`
    /// equivalent). `None` when the glob has no GP.
    pub fn line(&self) -> Option<u32> {
        let gp = self.gp();
        if gp.is_null() {
            return None;
        }
        Some(unsafe { (*gp).gp_line() as u32 })
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
