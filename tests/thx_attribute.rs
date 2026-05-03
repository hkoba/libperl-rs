//! End-to-end check for the `#[thx]` attribute proc-macro.
//!
//! On a threaded build, `#[thx]` should add `my_perl: *mut PerlInterpreter`
//! as the first parameter; on a non-threaded build, it should leave the
//! function unchanged.
//!
//! We verify by *calling* the resulting function with the appropriate
//! argument list. If the wrong number of args were generated, this won't
//! compile.

use libperl_rs::{thx, Perl, PerlInterpreter};

#[thx]
fn record_call(seen: &mut Option<*mut PerlInterpreter>, sentinel: i32) -> i32 {
    // In threaded build, `my_perl` is in scope here (injected by #[thx]).
    // In non-threaded build, it isn't — we don't reference it, so the
    // body works in both modes.
    #[cfg(perl_useithreads)]
    {
        *seen = Some(my_perl);
    }
    #[cfg(not(perl_useithreads))]
    {
        let _ = seen;
    }
    sentinel
}

#[test]
fn thx_injects_my_perl_in_threaded_build() {
    let perl = Perl::new();
    let my_perl = perl.as_ptr();
    let mut seen: Option<*mut PerlInterpreter> = None;

    #[cfg(perl_useithreads)]
    let rc = record_call(my_perl, &mut seen, 42);

    #[cfg(not(perl_useithreads))]
    let rc = {
        let _ = my_perl; // no-arg form in non-threaded
        record_call(&mut seen, 42)
    };

    assert_eq!(rc, 42);

    #[cfg(perl_useithreads)]
    assert_eq!(seen, Some(my_perl), "threaded build should have captured my_perl");

    #[cfg(not(perl_useithreads))]
    assert!(seen.is_none(), "non-threaded build should not touch `seen`");
}
