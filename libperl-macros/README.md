# libperl-macros

Procedural macros for [`libperl-rs`][rs]. Lets you write a Perl XS
extension module in pure Rust:

```rust
use libperl_rs::{xs_boot, xs_sub, IV, Perl, Rv, Sv, Av};

#[xs_sub]
fn make_pair(my_perl: &Perl) -> Rv<Av> {
    let av = Av::new(my_perl);
    av.push(my_perl, Sv::new_iv(my_perl, 1));
    av.push(my_perl, Sv::new_iv(my_perl, 2));
    av.into_rv(my_perl)
}

xs_boot! {
    package = "Mytest";
    subs    = [make_pair];
}
```

`cargo build` produces a `.so` that Perl can `XSLoader::load`.

## What's exported

- `#[thx]` — function attribute that splices `my_perl: *mut PerlInterpreter`
  as the first parameter in threaded builds and is a no-op in non-threaded
  builds. One source compiles in both `MULTIPLICITY` modes.
- `#[xs_sub]` — turns a high-level Rust signature like
  `fn foo(my_perl: &Perl, av: &Av) -> IV { ... }` into a complete
  `extern "C"` XS-callable trampoline. Supports IV / UV / NV / bool /
  String / `&CStr` / `&str` / `*mut SV` / `Sv` / `&Av` / `&Hv` / `Vec<T>` /
  `Result<T, String>` / `Option<T>` / `Rv<Av>` / `Rv<Hv>`. Performs runtime
  type checks (`SvROK` + `SvTYPE`) on reference args and croaks with a
  human-readable message on mismatch.
- `xs_boot!` — declarative macro that emits the module's
  `boot_<package>` symbol, which Perl's loader calls to register the
  XS subs.

## Status

Pre-1.0. Tracks `libperl-rs` 0.4.0-alpha. See the workspace README in
the [main repository][repo] for the larger picture.

## License

`GPL-1.0-or-later OR Artistic-1.0-Perl`.

[rs]: https://crates.io/crates/libperl-rs
[repo]: https://github.com/hkoba/libperl-rs
