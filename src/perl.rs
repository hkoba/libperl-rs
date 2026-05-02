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
    args: Vec<CString>,
    env: Vec<CString>,
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
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    /// Raw pointer for FFI. The conventional name is `my_perl` at the
    /// call site — see `docs/plan/README.md` §3.8 for naming rules.
    #[inline]
    pub fn as_ptr(&self) -> *mut PerlInterpreter {
        self.my_perl.as_ptr()
    }

    /// `perl_parse` with an explicit args / envp slice.
    pub fn parse<S: AsRef<str>>(&mut self, args: &[S], envp: &[S]) -> i32 {
        self.args = args
            .iter()
            .map(|a| CString::new(a.as_ref()).unwrap())
            .collect();
        self.env = envp
            .iter()
            .map(|a| CString::new(a.as_ref()).unwrap())
            .collect();
        self.perl_parse_inner()
    }

    /// `perl_parse` driven from `std::env::args()` / `vars()`.
    pub fn parse_env_args(&mut self, args: env::Args, envp: env::Vars) -> i32 {
        self.args = args
            .map(|a| CString::new(a).unwrap())
            .collect();
        self.env = envp
            .map(|(k, v)| CString::new(format!("{k}={v}")).unwrap())
            .collect();
        self.perl_parse_inner()
    }

    fn perl_parse_inner(&mut self) -> i32 {
        unsafe {
            perl_parse(
                self.as_ptr(),
                Some(xs_init as XsInitFn),
                self.args.len() as c_int,
                make_argv(&self.args).as_ptr() as *mut *mut c_char,
                ensure_terminating_null(make_argv(&self.env)).as_ptr() as *mut *mut c_char,
            )
        }
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

// ─── small argv helpers ────────────────────────────────────────────

fn make_argv(args: &[CString]) -> Vec<*mut c_char> {
    args.iter().map(|a| a.as_ptr() as *mut c_char).collect()
}

fn ensure_terminating_null(mut argv: Vec<*mut c_char>) -> Vec<*mut c_char> {
    if argv.last().is_none_or(|p| !p.is_null()) {
        argv.push(ptr::null_mut());
    }
    argv
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
