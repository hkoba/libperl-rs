//! `Op` newtype — a non-null handle to a node of a Perl OP tree, plus
//! the two iterators every OP-tree walker needs: execution order
//! ([`OpNextIter`], the `op_next` chain) and tree order
//! ([`OpSiblingIter`], the `OpSIBLING` chain).
//!
//! This is the OP-tree slice of "Step 2" in `docs/plan/README.md`,
//! extracted from the raw layers that grew downstream
//! (perl-optree-analyzer `analyzer-capture/src/raw.rs` and
//! perl-LibPerlRs-PartialEval `partial-eval-engine/src/raw.rs`).
//!
//! Accessors delegate to macrogen-emitted official API where one
//! exists (`OpSIBLING`); the remaining hand-written struct reads are
//! the ones with no public C macro, same set as the `B` module uses:
//!
//! - `op_next`: direct member read (`B` does the same),
//! - `first()`: `cUNOPx(o)->op_first` equivalent behind an
//!   `OPf_KIDS` guard,
//! - `name()`: `OP_NAME` is on the macrogen skip list
//!   (`libperl-sys/skip-codegen.txt`), so it reads the `PL_op_name`
//!   table instead — again the `B` way.

use std::ffi::CStr;
use std::ptr::NonNull;

use libperl_sys::{OP, OPf_KIDS, PL_op_name, opcode, unop};

// `OPclass` (and `Perl_op_class`) first appeared in perl 5.26.
#[cfg(perlapi_ver26)]
use libperl_sys::OPclass;

use crate::{Cop, Perl};

/// Non-null pointer to a Perl `OP`. Same ABI as `*mut OP`.
///
/// Like [`Sv`](crate::Sv), an `Op` does not own its referent —
/// dropping it is a no-op. OP lifetimes follow their owning CV
/// (`perl_destruct` / `op_free` invalidate them), which the type does
/// not track; keep walks inside the scope where the CV is known live.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Op(NonNull<OP>);

impl Op {
    /// Wrap a raw OP pointer without checking for null.
    ///
    /// # Safety
    /// Caller must guarantee `p` is non-null and points to a valid OP
    /// for at least the lifetime of the resulting `Op`.
    #[inline]
    pub unsafe fn from_raw_unchecked(p: *const OP) -> Self {
        debug_assert!(!p.is_null(), "Op::from_raw_unchecked received a null pointer");
        Op(unsafe { NonNull::new_unchecked(p as *mut OP) })
    }

    /// Wrap a raw OP pointer, returning `None` on null input. Takes
    /// `*const OP` because that is what tree sources like
    /// [`Cv::root`](crate::Cv::root) / [`Cv::start`](crate::Cv::start)
    /// hand out.
    #[inline]
    pub fn from_raw(p: *const OP) -> Option<Self> {
        NonNull::new(p as *mut OP).map(Op)
    }

    /// Raw pointer for FFI calls.
    #[inline]
    pub fn as_ptr(&self) -> *mut OP {
        self.0.as_ptr()
    }

    /// The op's type as a plain integer. The underlying bitfield is
    /// `u16` on modern Perl but `u32` on 5.30 and older — normalising
    /// to `u32` here keeps callers version-portable.
    #[inline]
    pub fn op_type_raw(&self) -> u32 {
        unsafe { (*self.0.as_ptr()).op_type() as u32 }
    }

    /// The op's type as the `opcode` enum, or `None` for
    /// out-of-range values (custom ops).
    #[inline]
    pub fn opcode(&self) -> Option<opcode> {
        opcode::try_from(self.op_type_raw()).ok()
    }

    /// The op's name (`"nextstate"`, `"add"`, ...) from the
    /// `PL_op_name` table, or `None` for out-of-range op types.
    pub fn name(&self) -> Option<&'static str> {
        // Range-validate through the opcode enum first; indexing the
        // static table with an arbitrary op_type would walk off the end
        // for custom ops.
        self.opcode()?;
        let p = unsafe { PL_op_name[self.op_type_raw() as usize] };
        unsafe { CStr::from_ptr(p) }.to_str().ok()
    }

    /// `op_flags` (`OPf_KIDS` and friends).
    #[inline]
    pub fn flags(&self) -> u8 {
        unsafe { (*self.0.as_ptr()).op_flags }
    }

    /// Next op in execution order (`op_next`), or `None` at the end
    /// of the chain.
    #[inline]
    pub fn next(&self) -> Option<Op> {
        Op::from_raw(unsafe { (*self.0.as_ptr()).op_next })
    }

    /// Next sibling in tree order (official `OpSIBLING`: the
    /// `op_moresib` check terminates at the parent back-pointer on
    /// 5.26+ layouts), or `None` for the last sibling.
    #[inline]
    pub fn sibling(&self) -> Option<Op> {
        Op::from_raw(unsafe { libperl_sys::OpSIBLING(self.0.as_ptr()) })
    }

    /// First child (`cUNOPx(o)->op_first` equivalent), or `None` when
    /// the op has no kids (`OPf_KIDS` unset). No public C macro exists
    /// for this — the struct read matches what `B` does.
    #[inline]
    pub fn first(&self) -> Option<Op> {
        if (self.flags() as u32 & OPf_KIDS) == 0 {
            None
        } else {
            Op::from_raw(unsafe { (*(self.0.as_ptr() as *const unop)).op_first })
        }
    }

    /// Iterate this op's children in tree order (first child, then
    /// its siblings). Empty for kid-less ops.
    #[inline]
    pub fn kids(&self) -> OpSiblingIter {
        OpSiblingIter { cur: self.first() }
    }

    /// Iterate in execution order starting from (and including) this
    /// op, following `op_next` until null.
    ///
    /// Note: the static `op_next` chain of a finished sub is not
    /// acyclic — loop constructs point back to their condition — so an
    /// unbounded walk over arbitrary code may not terminate. Cap with
    /// `.take(n)` unless the code is known to be straight-line.
    #[inline]
    pub fn next_iter(&self) -> OpNextIter {
        OpNextIter { cur: Some(*self) }
    }

    /// The op's class (`Perl_op_class`, the same classification `B`
    /// exposes as `B::class`).
    ///
    /// Only on perl 5.26+ — `OPclass` and `op_class()` were both born
    /// there. Version-portable callers that only need COP detection
    /// should use [`Op::as_cop`] instead, which works on every
    /// supported perl.
    #[cfg(perlapi_ver26)]
    #[inline]
    pub fn class(&self, perl: &Perl) -> OPclass {
        unsafe { crate::thx_call!(perl, Perl_op_class, self.0.as_ptr()) }
    }

    /// Whether this op is a COP (`nextstate` / `dbstate`), including
    /// an optimized-away ex-COP (`OP_NULL` whose `op_targ` records the
    /// original type — the same mapping `op_class` applies).
    fn is_cop(&self, perl: &Perl) -> bool {
        #[cfg(perlapi_ver26)]
        {
            self.class(perl) == OPclass::OPclass_COP
        }
        #[cfg(not(perlapi_ver26))]
        {
            let _ = perl;
            let mut t = self.op_type_raw();
            if t == opcode::OP_NULL as u32 {
                t = unsafe { (*self.0.as_ptr()).op_targ } as u32;
            }
            t == opcode::OP_NEXTSTATE as u32 || t == opcode::OP_DBSTATE as u32
        }
    }

    /// View this op as a [`Cop`] when it is one (`nextstate` /
    /// `dbstate`), giving access to its file / line.
    #[inline]
    pub fn as_cop(&self, perl: &Perl) -> Option<Cop> {
        if self.is_cop(perl) {
            Cop::from_raw(self.0.as_ptr() as *const libperl_sys::COP)
        } else {
            None
        }
    }
}

/// Execution-order iterator (`op_next` chain), yielded by
/// [`Op::next_iter`]. See the cycle caveat there.
pub struct OpNextIter {
    cur: Option<Op>,
}

impl Iterator for OpNextIter {
    type Item = Op;

    fn next(&mut self) -> Option<Op> {
        let op = self.cur?;
        self.cur = op.next();
        Some(op)
    }
}

/// Tree-order sibling iterator (`OpSIBLING` chain), yielded by
/// [`Op::kids`].
pub struct OpSiblingIter {
    cur: Option<Op>,
}

impl Iterator for OpSiblingIter {
    type Item = Op;

    fn next(&mut self) -> Option<Op> {
        let op = self.cur?;
        self.cur = op.sibling();
        Some(op)
    }
}
