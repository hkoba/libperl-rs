# libperl-sys

Low-level, raw FFI declarations for the Perl 5 C API (`libperl`).
Generated at build time by `bindgen` + [`libperl-macrogen`][m] (the
latter handles C macros and `static inline` functions that `bindgen`
deliberately skips).

This crate is the unsafe foundation under [`libperl-rs`][rs]; most
users want that safer wrapper. Reach for `libperl-sys` directly when
you need an API element that hasn't been wrapped yet, or when you're
writing a sibling crate at the same layer.

## What you get

Re-exported at the crate root:

- `Perl_*` extern functions and `PL_*` mutable statics (from bindgen).
- `Sv*` / `Av*` / `Hv*` macro-shaped helpers and inline wrappers (from
  libperl-macrogen).
- `PL_xxx!()` declarative macros that paper over the threaded vs
  non-threaded differences in how `PL_*` globals are spelled, so
  source like `PL_stack_base!(my_perl)` compiles in both modes.
- Opcode → name lookup table (`conv_opcode`) and per-function
  signature dictionary (`sigdb`) for downstream codegen.

## Safety

Every public item here is `unsafe` to use. Even reading a `PL_*`
global requires the right interpreter context, and Perl's API uses
raw `*mut` pointers ubiquitously.

## Build requirements

- A working Perl 5 install with development headers (`perl-dev` /
  `perl-devel`).
- LLVM + libclang (for `bindgen`).
- Internet access at first build (libperl-macrogen downloads a
  pre-extracted apidoc snapshot from GitHub Releases).

Threaded vs non-threaded Perl is auto-detected — no feature flag.

## License

`GPL-1.0-or-later OR Artistic-1.0-Perl`. Same terms as Perl 5 itself.

[m]: https://crates.io/crates/libperl-macrogen
[rs]: https://crates.io/crates/libperl-rs
