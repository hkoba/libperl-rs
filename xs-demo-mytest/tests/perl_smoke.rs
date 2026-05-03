//! Cargo-test bridge that runs the Perl-side `t/*.t` suite via `prove`.
//!
//! `cargo test` does *not* uplift cdylibs to `target/<profile>/lib<NAME>.so`
//! — the artifact only lives at `deps/lib<NAME>-<hash>.so` if it exists at
//! all. (`cargo test` may even skip building the cdylib entirely when the
//! tests don't link against it as a Rust library.) So we explicitly invoke
//! `cargo build -p Mytest` here to produce a known-location `.so`, then
//! stage it into `blib/` and invoke `prove`.
//!
//! Recursive `cargo` invocation is safe: cargo's per-target-dir lock is
//! reentrant from a child process, and `cargo build` is a no-op when the
//! artifact is up to date (typical second-run cost: ~50 ms).
//!
//! Skipped automatically if `prove` is not on PATH (e.g. minimal CI).

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const MOD_NAME: &str = "Mytest";

#[test]
fn perl_t_suite_passes() {
    if Command::new("prove").arg("--version").output().is_err() {
        eprintln!("prove not found on PATH — skipping Perl smoke test");
        return;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("crate dir must have a workspace parent");

    let so_src = build_cdylib(workspace_root);

    // Stage into blib/ exactly the way XSLoader expects.
    let blib_arch = manifest_dir.join(format!("blib/arch/auto/{MOD_NAME}"));
    let blib_lib = manifest_dir.join("blib/lib");
    std::fs::create_dir_all(&blib_arch).expect("mkdir blib/arch/auto/<mod>");
    std::fs::create_dir_all(&blib_lib).expect("mkdir blib/lib");
    std::fs::copy(&so_src, blib_arch.join(format!("{MOD_NAME}.so")))
        .expect("copy .so into blib/arch");
    std::fs::copy(
        manifest_dir.join(format!("perllib/{MOD_NAME}.pm")),
        blib_lib.join(format!("{MOD_NAME}.pm")),
    )
    .expect("copy .pm into blib/lib");

    let status = Command::new("prove")
        .current_dir(&manifest_dir)
        .args(["-Iblib/lib", "-Iblib/arch", "t/"])
        .status()
        .expect("failed to spawn prove");

    assert!(status.success(), "prove failed (exit {status})");
}

/// Run `cargo build -p Mytest` and return the absolute path of the
/// resulting cdylib (`target/<profile>/lib<NAME>.so`).
///
/// We use `cargo build` (not `cargo rustc`) because it deduplicates
/// against the parent `cargo test`'s build graph and uplifts the
/// cdylib to the user-visible `target/<profile>/` location, where
/// staging into `blib/` is trivial.
fn build_cdylib(workspace_root: &Path) -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let so = workspace_root
        .join("target")
        .join(profile)
        .join(format!("lib{MOD_NAME}.so"));

    // Fast path: a previous `cargo build` (or this test, on rerun)
    // already produced the artifact. cargo would do a 10+ second
    // dependency walk just to confirm "up to date", so short-circuit.
    if so.exists() {
        return so;
    }

    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = Command::new(&cargo);
    cmd.current_dir(workspace_root)
        .args(["build", "-p", MOD_NAME]);
    if profile == "release" {
        cmd.arg("--release");
    }
    let status = cmd.status().expect("failed to spawn cargo build");
    assert!(status.success(), "`cargo build -p {MOD_NAME}` failed");

    assert!(
        so.exists(),
        "cdylib not produced by cargo build at {}",
        so.display()
    );
    so
}
