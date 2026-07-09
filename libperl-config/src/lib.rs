//! # libperl-config
//!
//! Build-time helper for crates that link against `libperl`. Reads the
//! local Perl installation's `Config.pm` (via `perl -V:...`) and turns
//! the answers into `cargo:` directives + `cfg(...)` flags.
//!
//! Used by [`libperl-sys`](https://docs.rs/libperl-sys) and
//! [`libperl-rs`](https://docs.rs/libperl-rs) build scripts to:
//!
//! - emit `cargo:rustc-link-search` / `rustc-link-lib` / `rustc-link-arg`
//!   from `$Config{ccopts}` and `$Config{ldopts}`,
//! - expose feature toggles like `cfg(perl_useithreads)` based on
//!   `$Config{useithreads}`,
//! - emit per-API-version cfgs (`cfg(perlapi_ver26)` ...
//!   `cfg(perlapi_ver42)`) so source can branch on Perl version.
//!
//! Typical usage in a downstream `build.rs`:
//!
//! ```no_run
//! use libperl_config::PerlConfig;
//!
//! fn main() {
//!     let config = PerlConfig::default();
//!     config.emit_cargo_ldopts();
//!     config.emit_features(&["useithreads"]);
//!     config.emit_all_perlapi_versions(10);
//! }
//! ```
//!
//! ## Selecting which perl to build against
//!
//! By default the `perl` found on `PATH` is used. Set the `PERL`
//! environment variable to an absolute path to pick a specific
//! interpreter — e.g. an ExtUtils::MakeMaker postamble runs
//! `PERL=$(FULLPERL) cargo build ...` so the perl that ran Makefile.PL
//! and the perl being linked against are the same. Build scripts are
//! automatically re-run when `PERL` changes.
//!
//! See [`PerlConfig`] for the full API.

mod perl_command;
pub use perl_command::*;

mod perl_config;
pub use perl_config::*;

pub mod process_util;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let cfg = super::PerlConfig::default();
        assert!(cfg.read_ccopts().unwrap().len() > 0);
        assert!(cfg.read_ldopts().unwrap().len() > 0);
    }
    
    #[test]
    fn can_read_config() {
        let cfg = super::PerlConfig::default();
        let perl_version = cfg.dict.get("PERL_VERSION");
        assert_ne!(perl_version, None);
        if let Some(ver) = perl_version {
            let script = r#"
use strict;
use Config;
print "PERL_VERSION\t", $Config{PERL_VERSION};
"#;
            assert_eq!(super::process_util::process_command_output(
                cfg.command(&["-e", script]).output().unwrap()
            ).unwrap(), ["PERL_VERSION", ver].join("\t"))
        }
    }
}
