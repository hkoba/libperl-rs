# xs-demo-mytest — end-to-end demo of `#[xs_sub]` + `xs_boot!`

This crate demonstrates writing a Perl XS module **purely in Rust**, using
the `#[xs_sub]` attribute and `xs_boot!` macro from `libperl-macros`.

## What it shows

The entire user-visible source (`src/lib.rs`) is:

```rust
use libperl_rs::{xs_boot, xs_sub, IV};

#[xs_sub]
fn is_even(n: IV) -> bool {
    n % 2 == 0
}

xs_boot! {
    package = "Mytest";
    subs = [is_even];
}
```

This compiles to a `cdylib` that is loadable into Perl via `XSLoader`.
For comparison, the original hand-written version at
<https://github.com/hkoba/exp-libperl-rs-xs1> required ~80 lines of
manual stack-pointer arithmetic, mark/PUSHMARK/POPMARK manipulation, and
boot-function bookkeeping for the same single sub.

## Build / install / test

```sh
make test          # build .so, stage into blib/, run prove against t/
```

Equivalent of:

```sh
cargo build -p Mytest
cp ../target/debug/libMytest.so blib/arch/auto/Mytest/Mytest.so
cp perllib/Mytest.pm blib/lib/Mytest.pm
prove -Iblib/lib -Iblib/arch t/
```

Expected output: `All tests successful. Files=1, Tests=8.`

## Layout

```
xs-demo-mytest/
├── Cargo.toml          name="Mytest", crate-type=["cdylib"]
├── build.rs            emit_cargo_ldopts + perl_useithreads cfg
├── src/lib.rs          #[xs_sub] is_even + xs_boot!
├── perllib/Mytest.pm   thin wrapper, XSLoader::load
├── Makefile            build / install / test targets
└── t/01_is_even.t      Test::More: 6 result checks + 2 arity-error checks
```

## What's not yet supported

Step 3.6 (planned-next) will add types: `UV`, `NV`, `&str`, `&CStr`,
`*mut SV`, `Result<T, String>` (Err → `Perl_croak`).
