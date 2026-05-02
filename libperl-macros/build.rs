//! build.rs for libperl-macros (proc-macro crate).
//!
//! Detects the target Perl's threading mode via `libperl-config` and emits
//! `cfg(perl_useithreads)` so that the proc-macro source can branch on the
//! THX-needs-`my_perl` decision at proc-macro compile time.

use libperl_config::*;

fn main() {
    let config = PerlConfig::default();
    config.emit_features(&["useithreads"]);
}
