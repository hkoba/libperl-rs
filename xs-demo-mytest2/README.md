# xs-demo-mytest2 — Phase 3.8 demo (perlxstut EXAMPLE 4)

Demonstrates `#[xs_sub]` features added in Phase 3.8:

  * `&CStr` and `&str` argument types (T_PV equivalents)
  * `String` return type (with UTF-8 flag set on the result SV)
  * Mixing `IV` / `NV` / string args/returns

User-visible source (`src/lib.rs` ~30 lines):

```rust
#[xs_sub]
fn foo(i: IV, l: IV, s: &CStr) -> NV {
    (i + l + s.to_bytes().len() as IV) as NV
}

#[xs_sub]
fn shout(input: &str) -> String { input.to_uppercase() }

#[xs_sub]
fn byte_len(s: &CStr) -> IV { s.to_bytes().len() as IV }

xs_boot! { package = "Mytest2"; subs = [foo, shout, byte_len]; }
```

## Build / install / test

```sh
make test          # cargo build + stage into blib/ + prove t/
```

## Notes

* `&str` arguments call `to_str()` on the underlying `&CStr`; on
  invalid UTF-8 the trampoline calls `Perl_croak` instead of
  panicking.
* `String` returns set the SV's `SVf_UTF8` flag because `String`
  always holds valid UTF-8.
* Both args borrow the SV's PV buffer for the duration of the call;
  the user body must not stash the slice anywhere that outlives the
  trampoline.
