//! Capture-stderr-based assertions on `Perl::parse` failure paths.
//!
//! Perl writes parse / runtime diagnostics directly to file descriptor 2
//! via the C runtime — Rust's `cargo test` harness only captures
//! `println!`/`eprintln!` (Rust-side stdio), not foreign writes. So a test
//! that triggers `Global symbol "$foo" requires explicit package name`
//! leaks the message to the host terminal and, on GitHub Actions, gets
//! picked up as an `error?` annotation on the build.
//!
//! The helper here redirects fd 2 to a tempfile around a closure, then
//! reads the captured bytes back. With this in hand we can assert both:
//!
//!   * `perl.parse` returned a non-zero exit code
//!   * the captured stderr contained the diagnostic substring
//!
//! and the original CI-annotation noise is gone as a side effect.

use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::sync::Mutex;

use libperl_proto0::Perl;

/// Serialize tests in this binary. Two reasons:
///
///   1. `dup2(tmpfd, 2)` is a per-process operation. Two threads racing
///      to redirect fd 2 onto different tempfiles will see each other's
///      writes, making the captured-stderr assertions flaky.
///   2. `Perl::new()` constructs a Perl interpreter, which keeps a lot
///      of process-global state (PL_*) that is not safe to operate on
///      from multiple threads concurrently.
static SERIAL: Mutex<()> = Mutex::new(());

/// Run `f`, capturing anything that gets written to fd 2 during the call,
/// and return it as a UTF-8 lossy string. Restores stderr on the way out.
///
/// Not thread-safe — callers should run in serial (the default for
/// integration tests, which each get their own binary).
fn capture_stderr<F: FnOnce()>(f: F) -> String {
    // Make sure any pending Rust-side stderr buffering is out of the way
    // before we start swapping fd 2 underneath the C runtime.
    let _ = std::io::Write::flush(&mut std::io::stderr());

    let mut tmp = tempfile::tempfile().expect("create tempfile for stderr capture");

    // Save the real fd 2 so we can put it back afterwards.
    let saved = unsafe { libc::dup(2) };
    assert!(saved >= 0, "dup(2) failed: {}", std::io::Error::last_os_error());

    // Point fd 2 at the tempfile. Both the C runtime (via stderr) and any
    // direct write(2) calls now land in the file.
    let rc = unsafe { libc::dup2(tmp.as_raw_fd(), 2) };
    assert!(rc >= 0, "dup2 failed: {}", std::io::Error::last_os_error());

    // Run the closure with stderr redirected.
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // Make sure C-runtime buffers (and Perl's own PerlIO) are flushed
    // before we swap fd 2 back.
    unsafe { libc::fflush(std::ptr::null_mut()) };

    // Restore the original stderr.
    let restore_rc = unsafe { libc::dup2(saved, 2) };
    assert!(restore_rc >= 0, "dup2 (restore) failed: {}", std::io::Error::last_os_error());
    unsafe { libc::close(saved) };

    // Read everything that landed in the tempfile.
    tmp.seek(SeekFrom::Start(0)).expect("rewind tempfile");
    let mut bytes = Vec::new();
    tmp.read_to_end(&mut bytes).expect("read tempfile back");

    // Re-raise any panic from `f` after we've done cleanup.
    if let Err(payload) = panic {
        std::panic::resume_unwind(payload);
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

/// `use strict; $foo` should fail to parse with the expected diagnostic.
#[test]
fn strict_undeclared_var_emits_error() {
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let mut perl = Perl::new();
    let mut rc: i32 = 0;
    let stderr = capture_stderr(|| {
        rc = perl.parse(&["", "-e", r#"use strict; $foo"#], &[""]);
    });

    assert_ne!(
        rc, 0,
        "perl.parse should signal failure on `use strict; $foo` (rc={rc}, stderr={stderr:?})"
    );
    assert!(
        stderr.contains("requires explicit package name"),
        "expected diagnostic missing from captured stderr: {stderr:?}"
    );
    // The diagnostic should also name the offending symbol.
    assert!(
        stderr.contains("$foo"),
        "captured stderr did not mention `$foo`: {stderr:?}"
    );
}

/// Trivial sanity check: a clean `print "ok"` script should parse with rc==0
/// and emit nothing on stderr.
#[test]
fn well_formed_script_is_silent_on_stderr() {
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let mut perl = Perl::new();
    let mut rc: i32 = 1; // sentinel — assert it was overwritten
    let stderr = capture_stderr(|| {
        rc = perl.parse(&["", "-e", r#"my $x = 1 + 2;"#], &[""]);
    });
    assert_eq!(rc, 0, "well-formed script should parse cleanly (stderr={stderr:?})");
    assert!(stderr.is_empty(), "unexpected stderr from clean parse: {stderr:?}");
}
