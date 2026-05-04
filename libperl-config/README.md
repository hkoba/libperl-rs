# libperl-config

Build-time helper for crates that link against `libperl`. Reads the
local Perl installation's `Config.pm` (via `perl -V:...`) and turns
the answers into `cargo:` directives + `cfg(...)` flags.

Used by [`libperl-sys`][sys] and [`libperl-rs`][rs] build scripts.

## Usage

```rust
// build.rs
use libperl_config::PerlConfig;

fn main() {
    let config = PerlConfig::default();
    config.emit_cargo_ldopts();              // link flags from $Config{ldopts}
    config.emit_features(&["useithreads"]);  // cfg(perl_useithreads) if set
    config.emit_all_perlapi_versions(10);    // cfg(perlapi_ver26)..ver42
}
```

The dependent crate's source can then write things like:

```rust
#[cfg(perl_useithreads)]
fn foo(my_perl: *mut PerlInterpreter) { ... }

#[cfg(not(perl_useithreads))]
fn foo() { ... }
```

## License

`GPL-1.0-or-later OR Artistic-1.0-Perl`. Same terms as Perl 5 itself.

[sys]: https://crates.io/crates/libperl-sys
[rs]: https://crates.io/crates/libperl-rs
