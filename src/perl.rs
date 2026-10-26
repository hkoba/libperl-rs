//! `Perl` — RAII-managed wrapper around `*mut PerlInterpreter`.
//!
//! See `docs/plan/README.md` §3.4 for the design rationale (`NonNull` to
//! encode the non-null invariant at the safe boundary while keeping
//! pointer-style aliasing for the FFI layer).

use std::env;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::ptr::NonNull;

use libperl_sys::{CV, PerlInterpreter, Perl_newXS, perl_alloc, perl_construct, perl_destruct, perl_parse};

/// A live Perl interpreter. Allocated by `perl_alloc` and torn down by
/// `perl_destruct` on drop.
///
/// The `my_perl` field is `NonNull<PerlInterpreter>` so that the
/// "interpreter is never null" invariant is encoded in the type.
/// FFI calls extract a raw pointer via [`Perl::as_ptr`] — that's the
/// boundary where Rust's safe-typed world meets the C ABI.
pub struct Perl {
    my_perl: NonNull<PerlInterpreter>,
    /// What `perl_parse` was given; perl keeps pointing at both for
    /// the interpreter's whole life (see [`ArgBlock`]).
    args: ArgBlock,
    env: ArgBlock,
}

// `NonNull<T>` is automatically `!Send !Sync`, which matches the Perl
// convention of "1 interpreter = 1 thread". No `unsafe impl Send/Sync`
// is provided.

impl Perl {
    /// Allocate and construct a fresh interpreter. Panics on allocation
    /// failure (typically OOM, very rare).
    pub fn new() -> Self {
        let raw = unsafe { perl_alloc() };
        let my_perl = NonNull::new(raw)
            .expect("perl_alloc returned null (out of memory?)");
        unsafe { perl_construct(my_perl.as_ptr()) };
        Perl {
            my_perl,
            args: ArgBlock::empty(),
            env: ArgBlock::empty(),
        }
    }

    /// Raw pointer for FFI. The conventional name is `my_perl` at the
    /// call site — see `docs/plan/README.md` §3.8 for naming rules.
    #[inline]
    pub fn as_ptr(&self) -> *mut PerlInterpreter {
        self.my_perl.as_ptr()
    }

    /// Wrap a raw `*mut PerlInterpreter` as a borrowed `Perl`.
    ///
    /// # Safety
    /// - `p` must point to a live, valid interpreter.
    /// - The returned `Perl` MUST NOT be dropped: its `Drop` runs
    ///   `perl_destruct`, tearing down an interpreter this constructor
    ///   does not own. The intended usage is to wrap in
    ///   `core::mem::ManuallyDrop` immediately. The `#[xs_sub]`
    ///   proc-macro does this when a body declares a `my_perl: &Perl`
    ///   first parameter.
    pub unsafe fn from_raw_unchecked(p: *mut PerlInterpreter) -> Self {
        // Non-threaded builds: the `#[xs_sub]` proc-macro passes a
        // null `my_perl` stub here. That's fine — every FFI call goes
        // through `thx_call!`, which in non-threaded mode discards
        // the `Perl` argument before invoking the bare libperl-sys
        // function. So `as_ptr()` is never actually dereferenced;
        // a dangling sentinel works as a placeholder.
        //
        // Threaded builds: a null pointer here is a programming
        // error (callers must hand over a live interpreter). Callers
        // get the same `dangling()` sentinel rather than a panic, so
        // the failure surfaces at the first FFI deref instead of at
        // construction — matches the rest of `unsafe`'s "garbage in,
        // segfault out" contract.
        let my_perl = NonNull::new(p).unwrap_or(NonNull::dangling());
        Perl {
            my_perl,
            args: ArgBlock::empty(),
            env: ArgBlock::empty(),
        }
    }

    /// `perl_parse` with an explicit args / envp slice.
    ///
    /// Both slices are copied into this `Perl` and stay valid until it
    /// is dropped: perl keeps `PL_origargv` / `PL_origenviron` pointing
    /// at them, and `$0 = ...` writes the new process title over the
    /// argument strings (truncated to their total length, as in a real
    /// perl process). Panics on an interior NUL byte, like `CString`.
    pub fn parse<S: AsRef<str>>(&mut self, args: &[S], envp: &[S]) -> i32 {
        self.args = ArgBlock::new(args.iter().map(|a| a.as_ref()));
        self.env = ArgBlock::new(envp.iter().map(|a| a.as_ref()));
        self.perl_parse_inner()
    }

    /// `perl_parse` driven from `std::env::args()` / `vars()`.
    pub fn parse_env_args(&mut self, args: env::Args, envp: env::Vars) -> i32 {
        let args: Vec<String> = args.collect();
        let envp: Vec<String> = envp.map(|(k, v)| format!("{k}={v}")).collect();
        self.parse(&args, &envp)
    }

    fn perl_parse_inner(&mut self) -> i32 {
        let argc = self.args.len() as c_int;
        let argv = self.args.as_mut_ptr();
        let envp = self.env.as_mut_ptr();
        unsafe { perl_parse(self.as_ptr(), Some(xs_init as XsInitFn), argc, argv, envp) }
    }

    /// `perl_run` — execute the parsed program's main body.
    ///
    /// Note the split this makes explicit: [`Perl::parse`] only
    /// *compiles* the program (`BEGIN` / `use` run, the main body
    /// does not), which is exactly the "parse-only" mode an
    /// introspection client wants — after `parse`, the whole
    /// symbol-table / OP-tree surface below is available without
    /// having executed the program. Call `run` only when execution
    /// is actually desired.
    pub fn run(&mut self) -> i32 {
        unsafe { libperl_sys::perl_run(self.as_ptr()) }
    }

    /// `gv_stashpv` — look up the stash (symbol table hash) of
    /// package `pack`, e.g. `"main"` or `"Foo::Bar"`. `None` when the
    /// package does not exist (with `flags = 0`; pass `GV_ADD`-style
    /// flags to autovivify).
    pub fn gv_stashpv(&self, pack: &str, flags: i32) -> Option<crate::Hv> {
        let name = CString::new(pack).ok()?;
        let hv = unsafe {
            crate::thx_call!(self, Perl_gv_stashpv, name.as_ptr(), flags as _)
        };
        if hv.is_null() {
            None
        } else {
            Some(unsafe { crate::Hv::from_raw_unchecked(hv) })
        }
    }

    /// `get_sv` — look up a package scalar by name (`"main::0"`,
    /// `"Foo::bar"`; unqualified names resolve in `main`). `None`
    /// when it does not exist (with `flags = 0`).
    pub fn get_sv(&self, name: &str, flags: i32) -> Option<crate::Sv> {
        let name = CString::new(name).ok()?;
        let sv = unsafe {
            crate::thx_call!(self, Perl_get_sv, name.as_ptr(), flags as _)
        };
        crate::Sv::from_raw(sv)
    }
}

impl Default for Perl {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Perl {
    fn drop(&mut self) {
        unsafe { perl_destruct(self.as_ptr()) };
    }
}

// ─── xs_init / DynaLoader bootstrap ────────────────────────────────

unsafe extern "C" {
    #[cfg(perl_useithreads)]
    fn boot_DynaLoader(perl: *mut PerlInterpreter, cv: *mut CV);
    #[cfg(not(perl_useithreads))]
    fn boot_DynaLoader(cv: *mut CV);
}

#[cfg(perl_useithreads)]
type XsInitFn = extern "C" fn(*mut PerlInterpreter);
#[cfg(not(perl_useithreads))]
type XsInitFn = extern "C" fn();

#[cfg(perl_useithreads)]
extern "C" fn xs_init(my_perl: *mut PerlInterpreter) {
    let name = c"DynaLoader::boot_DynaLoader".as_ptr();
    let file = c"libperl-rs".as_ptr();
    unsafe { Perl_newXS(my_perl, name, Some(boot_DynaLoader), file) };
}

#[cfg(not(perl_useithreads))]
extern "C" fn xs_init() {
    let name = c"DynaLoader::boot_DynaLoader".as_ptr();
    let file = c"libperl-rs".as_ptr();
    unsafe { Perl_newXS(name, Some(boot_DynaLoader), file) };
}

// ─── argv / envp storage ───────────────────────────────────────────

/// Strings for `perl_parse`, laid out like a process's real `argv` /
/// `environ`: one contiguous block of NUL-terminated strings plus a
/// NULL-terminated pointer array.
///
/// perl does not copy either. `PL_origargv` / `PL_origenviron` keep
/// pointing at them for the interpreter's whole life, and assigning
/// `$0` (`S_set_dollarzero` in mg.c) writes the new process title over
/// `PL_origargv[0]` and sets `PL_origargv[1..]` to NULL. `perl_parse`
/// sizes that scratch area (`PL_origalen`) by how far the argument
/// strings sit back to back, with a heuristic that also accepts gaps
/// of up to a pointer size — separately allocated `CString`s could be
/// taken for contiguous by accident and then overwritten past their
/// ends. Keeping every string in one block, and the pointer array
/// owned and writable, makes those writes land where perl expects.
struct ArgBlock {
    /// The strings, each with its NUL; empty for no strings. Never
    /// read from Rust — it is the storage `ptrs` points into.
    #[allow(dead_code)]
    bytes: Box<[u8]>,
    /// One pointer per string into `bytes`, then NULL.
    ptrs: Vec<*mut c_char>,
}

impl ArgBlock {
    fn empty() -> Self {
        ArgBlock {
            bytes: Box::new([]),
            ptrs: vec![ptr::null_mut()],
        }
    }

    fn new<'a>(items: impl Iterator<Item = &'a str>) -> Self {
        let mut bytes = Vec::new();
        let mut starts = Vec::new();
        for s in items {
            // CString validates the interior-NUL rule (panics like the
            // previous `CString::new(..).unwrap()` did).
            let c = CString::new(s).unwrap();
            starts.push(bytes.len());
            bytes.extend_from_slice(c.as_bytes_with_nul());
        }
        // Box first: the pointers must refer to the final heap block.
        let bytes: Box<[u8]> = bytes.into_boxed_slice();
        let base = bytes.as_ptr() as *mut c_char;
        let mut ptrs: Vec<*mut c_char> = starts.iter().map(|&o| base.wrapping_add(o)).collect();
        ptrs.push(ptr::null_mut());
        ArgBlock { bytes, ptrs }
    }

    /// Number of strings (`argc`).
    fn len(&self) -> usize {
        self.ptrs.len() - 1
    }

    /// The NULL-terminated pointer array, for `perl_parse`.
    fn as_mut_ptr(&mut self) -> *mut *mut c_char {
        self.ptrs.as_mut_ptr()
    }
}

// ─── perl_call! macro ──────────────────────────────────────────────

/// Wrap a `Perl_*` (bindgen) function call so the source compiles
/// against both threaded and non-threaded Perl without `cfg`.
///
/// In threaded builds, `$my_perl` is prepended as the first argument.
/// In non-threaded builds, `$my_perl` is type-checked, evaluated once,
/// and discarded.
///
/// ```ignore
/// let my_perl = perl.as_ptr();
/// let cv = perl_call!(my_perl, Perl_newXS(name.as_ptr(), sub, file.as_ptr()));
/// ```
///
/// (See `docs/plan/README.md` §3.6 for the argument-form rationale and
/// hygiene constraints that prevent a no-arg variant.)
#[cfg(perl_useithreads)]
#[macro_export]
macro_rules! perl_call {
    ($my_perl:expr, $f:ident ( $($arg:expr),* $(,)? )) => {{
        let __my_perl: *mut $crate::PerlInterpreter = $my_perl;
        unsafe { $crate::$f(__my_perl, $($arg),*) }
    }};
}

#[cfg(not(perl_useithreads))]
#[macro_export]
macro_rules! perl_call {
    ($my_perl:expr, $f:ident ( $($arg:expr),* $(,)? )) => {{
        // type-check + evaluate-once for source portability with the
        // threaded form, then discard in non-threaded
        let _: *mut $crate::PerlInterpreter = $my_perl;
        unsafe { $crate::$f($($arg),*) }
    }};
}
