//! `Cv` newtype — a non-null handle to a Perl `CV` (code value).
//!
//! This is the first slice of "Step 2" in `docs/plan/README.md`
//! (Cv/Op newtypes for OP-tree walkers). The accessors below cover
//! what a static-analysis client needs to get from a coderef to its
//! OP tree: `root` / `start` / `padlist` / `file` / `proto`, plus the
//! `is_xsub` guard that must be checked before touching the
//! `xcv_root_u` / `xcv_padlist_u` unions.
//!
//! All accessors delegate to the macrogen-emitted official API
//! (`CvROOT`, `CvSTART`, `CvPADLIST`, `CvFILE`, `CvISXSUB`) — no
//! hand-written struct pokes. `proto()` composes `SvPOK` +
//! `SvPVX_const` + `SvCUR` because `CvPROTO` itself is on the
//! macrogen skip list (see `libperl-sys/skip-codegen.txt`).
//!
//! Like [`Sv`](crate::Sv), a `Cv` does **not** own its referent:
//! dropping it is a no-op. The `#[xs_sub]` proc-macro recognises a
//! bare `Cv` parameter as "caller must pass a CODE reference" and
//! generates the `SvROK` + `SVt_PVCV` check + croak in the
//! trampoline (see `xs_sub.rs` `ArgKind::InCvRef`).

use std::ptr::NonNull;

use libperl_sys::{CV, OP, PADLIST, SV, svtype};

use crate::{Cop, Gv, Op, PadNames, Perl};

/// Non-null pointer to a Perl `CV`. Same ABI as `*mut CV`.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Cv(NonNull<CV>);

impl Cv {
    /// Wrap a raw `*mut CV` without checking for null.
    ///
    /// # Safety
    /// Caller must guarantee `p` is non-null and points to a valid CV
    /// for at least the lifetime of the resulting `Cv`.
    #[inline]
    pub unsafe fn from_raw_unchecked(p: *mut CV) -> Self {
        debug_assert!(!p.is_null(), "Cv::from_raw_unchecked received a null pointer");
        Cv(unsafe { NonNull::new_unchecked(p) })
    }

    /// Wrap a raw `*mut CV`, returning `None` on null input.
    #[inline]
    pub fn from_raw(p: *mut CV) -> Option<Self> {
        NonNull::new(p).map(Cv)
    }

    /// Dereference a Perl-level coderef SV (`\&sub`, `sub {...}`)
    /// into its CV. Returns `None` when `sv` is null, not a
    /// reference, or references something other than a CODE value.
    #[inline]
    pub fn from_coderef(sv: *mut SV) -> Option<Cv> {
        if sv.is_null() || unsafe { libperl_sys::SvROK(sv) } == 0 {
            return None;
        }
        let target = unsafe { libperl_sys::SvRV(sv) };
        if unsafe { libperl_sys::SvTYPE(target) } != svtype::SVt_PVCV {
            return None;
        }
        Some(unsafe { Cv::from_raw_unchecked(target as *mut CV) })
    }

    /// Raw `*mut CV` for FFI calls.
    #[inline]
    pub fn as_ptr(&self) -> *mut CV {
        self.0.as_ptr()
    }

    /// True when this CV is an XSUB (C-implemented). XSUBs have no
    /// OP tree; `root` / `start` / `padlist` return null for them.
    #[inline]
    pub fn is_xsub(&self) -> bool {
        unsafe { libperl_sys::CvISXSUB(self.as_ptr() as *const _) != 0 }
    }

    /// Root of the CV's OP tree (`CvROOT`), or null for XSUBs.
    #[inline]
    pub fn root(&self) -> *const OP {
        if self.is_xsub() {
            std::ptr::null()
        } else {
            unsafe { libperl_sys::CvROOT(self.as_ptr() as *const _) }
        }
    }

    /// First OP in execution order (`CvSTART`), or null for XSUBs.
    #[inline]
    pub fn start(&self) -> *const OP {
        if self.is_xsub() {
            std::ptr::null()
        } else {
            unsafe { libperl_sys::CvSTART(self.as_ptr() as *const _) }
        }
    }

    /// The CV's PADLIST (lexical scratchpad), or null for XSUBs.
    #[inline]
    pub fn padlist(&self) -> *const PADLIST {
        if self.is_xsub() {
            std::ptr::null()
        } else {
            unsafe { libperl_sys::CvPADLIST(self.as_ptr() as *const _) }
        }
    }

    /// Source file the sub was compiled from (`CvFILE`);
    /// `"(eval N)"` for string-eval'd subs.
    pub fn file(&self) -> Option<String> {
        let p = unsafe { libperl_sys::CvFILE(self.as_ptr() as *const _) };
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

    /// [`Cv::root`] as an [`Op`] handle (`None` for XSUBs).
    #[inline]
    pub fn root_op(&self) -> Option<Op> {
        Op::from_raw(self.root())
    }

    /// [`Cv::start`] as an [`Op`] handle (`None` for XSUBs).
    #[inline]
    pub fn start_op(&self) -> Option<Op> {
        Op::from_raw(self.start())
    }

    /// The GV the sub was defined through (`CvGV`), if any. Gives
    /// access to the sub's package-qualified name and the glob's
    /// file / line.
    #[inline]
    pub fn gv(&self, perl: &Perl) -> Option<Gv> {
        // Inferred cast: the generated `CvGV` takes `*const SV` on
        // most perls but `*const CV` on exactly 5.32 (apidoc type
        // normalisation difference); `as *const _` fits both. Called
        // through `sys::thx` because THX-ness varies by version, not
        // just by build mode: on 5.20 the generated `CvGV` takes no
        // `my_perl` even on threaded perl (the thx shim discards it).
        let gv = unsafe { libperl_sys::thx::CvGV(perl.as_ptr(), self.as_ptr() as *const _) };
        Gv::from_raw(gv)
    }

    /// The sub's `(qualified, unqualified)` name pair
    /// (`("Foo::bar", "bar")`), resolved via [`Cv::gv`]. `None` for
    /// nameless subs.
    pub fn names(&self, perl: &Perl) -> Option<(String, String)> {
        let gv = self.gv(perl)?;
        let name = gv.name()?;
        let full = match gv.stash_name() {
            Some(pkg) => format!("{pkg}::{name}"),
            None => name.clone(),
        };
        Some((full, name))
    }

    /// The first COP (`nextstate`) in the sub's OP tree, in tree
    /// order — i.e. the sub's first statement, whose
    /// [`line`](Cop::line) / [`file`](Cop::file) locate the sub body
    /// in its source. `None` for XSUBs and bodiless subs.
    ///
    /// Walks tree order (preorder), not the `op_next` chain, so loop
    /// back-edges cannot cycle the search.
    pub fn first_cop(&self, perl: &Perl) -> Option<Cop> {
        let mut stack: Vec<Op> = self.root_op().into_iter().collect();
        while let Some(op) = stack.pop() {
            if let Some(cop) = op.as_cop(perl) {
                return Some(cop);
            }
            // Push the sibling below the first kid so the kid is
            // taken first (preorder).
            if let Some(sib) = op.sibling() {
                stack.push(sib);
            }
            if let Some(kid) = op.first() {
                stack.push(kid);
            }
        }
        None
    }

    /// Iterate the sub's lexical-name slots (pad names), in pad-offset
    /// order starting at offset 0. Empty for XSUBs. See
    /// [`PadNames`](crate::PadNames).
    #[inline]
    pub fn pad_names(&self) -> PadNames {
        PadNames::from_padlist(self.padlist())
    }

    /// The sub's prototype string (`CvPROTO`), if any. A CV stores
    /// its prototype in its own PV slot, so this is `SvPOK` +
    /// `SvPVX_const`/`SvCUR` on the CV itself.
    pub fn proto(&self) -> Option<String> {
        let sv = self.as_ptr() as *mut SV;
        unsafe {
            if libperl_sys::SvPOK(sv) == 0 {
                return None;
            }
            let pv = libperl_sys::SvPVX_const(sv);
            if pv.is_null() {
                return None;
            }
            let len = libperl_sys::SvCUR(sv);
            let bytes = std::slice::from_raw_parts(pv as *const u8, len as usize);
            Some(String::from_utf8_lossy(bytes).into_owned())
        }
    }
}
