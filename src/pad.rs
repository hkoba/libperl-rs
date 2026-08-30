//! Pad-name access — the lexical (`my` / `our`) variable names of a
//! CV, read from its PADLIST's name list. Step 2's "lexical pad
//! resolution" piece (`docs/plan/README.md` §4 Step 3.2), extracted
//! from libperl-proto0's `eg/pad0.rs` / example `102_padname_type.rs`
//! and perl-optree-analyzer's `raw.rs`.
//!
//! All accessors go through the macrogen-emitted official API
//! (`PadlistNAMES`, `PadnamelistMAX` / `PadnamelistARRAY`,
//! `PadnamePV` / `PadnameLEN` / `PadnameTYPE`). On perl 5.28/5.30
//! those `Padname*` accessors come from the hand-written compat block
//! in `libperl-sys/src/perl_core.rs` (macrogen still suppresses them
//! there); the call sites here are identical either way.
//!
//! Entry point: [`Cv::pad_names`](crate::Cv::pad_names).

use std::ptr::NonNull;

use libperl_sys::{PADLIST, PADNAME, PADNAMELIST};

/// Non-null pointer to a Perl `PADNAME` — one lexical's name slot.
/// Non-owning, like the other newtypes.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PadName(NonNull<PADNAME>);

impl PadName {
    /// Wrap a raw PADNAME pointer, returning `None` on null input.
    #[inline]
    pub fn from_raw(p: *const PADNAME) -> Option<Self> {
        NonNull::new(p as *mut PADNAME).map(PadName)
    }

    /// Raw pointer for FFI calls.
    #[inline]
    pub fn as_ptr(&self) -> *mut PADNAME {
        self.0.as_ptr()
    }

    /// The lexical's name including sigil (`"$x"`, `"@args"`, ...),
    /// or `None` for unnamed slots (targets, sub-op temporaries).
    pub fn pv(&self) -> Option<String> {
        let pv = unsafe { libperl_sys::PadnamePV(self.as_ptr()) };
        if pv.is_null() {
            return None;
        }
        #[cfg(perlapi_ver22)]
        let len = unsafe { libperl_sys::PadnameLEN(self.as_ptr()) };
        // 5.20 (PADNAME = SV 時代): 生成体 PadnameLEN は THX 付きで、
        // Perl ハンドルを持たないここからは呼べない。pad.h 5.20 の定義
        // `(pn == &PL_sv_undef ? 0 : SvCUR(pn))` の undef 分岐は直前の
        // PV null チェック (undef は POKp でない) で除外済みなので、
        // SvCUR 直読みで等価。
        #[cfg(not(perlapi_ver22))]
        let len = unsafe { libperl_sys::SvCUR(self.as_ptr() as *const libperl_sys::SV) };
        let bytes = unsafe { std::slice::from_raw_parts(pv as *const u8, len as usize) };
        Some(String::from_utf8_lossy(bytes).into_owned())
    }

    /// For `my Foo $x`-style typed lexicals: the type stash's name
    /// (`"Foo"`). `None` for untyped lexicals.
    pub fn type_stash_name(&self) -> Option<String> {
        let stash = unsafe { libperl_sys::PadnameTYPE(self.as_ptr()) };
        if stash.is_null() {
            return None;
        }
        let p = unsafe { libperl_sys::HvNAME(stash) };
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

/// Iterator over a CV's pad-name slots, yielded by
/// [`Cv::pad_names`](crate::Cv::pad_names). Each item corresponds to
/// one pad offset (starting at 0); `None` items are allocated but
/// nameless slots. Pair with `.enumerate()` when the pad offsets
/// matter.
pub struct PadNames {
    arr: *mut *mut PADNAME,
    ix: isize,
    max: isize,
}

impl PadNames {
    /// Build from a CV's PADLIST pointer (null-safe: a null padlist —
    /// e.g. an XSUB's — yields an empty iterator).
    pub(crate) fn from_padlist(pl: *const PADLIST) -> PadNames {
        let empty = PadNames {
            arr: std::ptr::null_mut(),
            ix: 0,
            max: -1,
        };
        if pl.is_null() {
            return empty;
        }
        let pnl: *const PADNAMELIST = unsafe { libperl_sys::PadlistNAMES(pl) };
        if pnl.is_null() {
            return empty;
        }
        PadNames {
            arr: unsafe { libperl_sys::PadnamelistARRAY(pnl) },
            ix: 0,
            // `PadnamelistMAX` is the last used index (xpadnl_fill),
            // -1 when empty — same convention as AvFILL.
            max: unsafe { libperl_sys::PadnamelistMAX(pnl) as isize },
        }
    }
}

impl Iterator for PadNames {
    type Item = Option<PadName>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ix > self.max || self.arr.is_null() {
            return None;
        }
        let p = unsafe { *self.arr.offset(self.ix) };
        self.ix += 1;
        Some(PadName::from_raw(p))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let r = (self.max + 1 - self.ix).max(0) as usize;
        (r, Some(r))
    }
}
