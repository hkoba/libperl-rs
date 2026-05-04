# libperl-rs

Embed the Perl 5 runtime inside a Rust application. A safe-ish wrapper
on top of [`libperl-sys`](https://crates.io/crates/libperl-sys) (the
raw FFI layer) and [`libperl-macros`](https://crates.io/crates/libperl-macros)
(`#[xs_sub]` / `xs_boot!` for writing XS modules in pure Rust).

## Workspace layout

| Crate | Role |
|---|---|
| [`libperl-rs`](./)                   | High-level Rust API. Start here. |
| [`libperl-sys`](./libperl-sys)       | Low-level FFI bindings (bindgen + libperl-macrogen). |
| [`libperl-macros`](./libperl-macros) | Proc-macros (`#[thx]`, `#[xs_sub]`, `xs_boot!`) for writing XS subs in Rust. |
| [`libperl-config`](./libperl-config) | Build-script helper that reads `Config.pm`. |
| `xs-demo-mytest` / `xs-demo-mytest2` | End-to-end demo XS modules used by the test suite. |

The auto-generated raw bindings (the C macros and `static inline`
functions that `bindgen` skips) come from a separate companion crate,
[`libperl-macrogen`](https://crates.io/crates/libperl-macrogen).

## Status

Pre-1.0. The 0.4.0-alpha series tracks the rebuild plan in
`docs/plan/README.md`; the public API may move between alpha
releases. CI exercises Perl 5.30 / 5.32 / 5.34 / 5.36 / 5.38 / 5.40 /
5.42 / latest, both threaded and non-threaded.

## Build requirements

- Perl 5 with development headers (`perl-dev` / `perl-devel`).
- LLVM + libclang (for `bindgen`).
- Internet access at first build (libperl-macrogen downloads a
  pre-extracted apidoc snapshot from GitHub Releases — see that
  crate's README).

## Quick test

```bash
cargo test --all --examples
```

For a multi-version sweep without polluting your local toolchain:

```bash
./runtest-docker.zsh --image=perl:5.40           # non-threaded
./runtest-docker.zsh --image=perl:5.40-threaded  # threaded
```

## License

Dual-licensed under the same terms as Perl 5 itself:
`GPL-1.0-or-later OR Artistic-1.0-Perl`.
