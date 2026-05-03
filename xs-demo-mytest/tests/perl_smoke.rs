//! Cargo-test bridge that runs the Perl-side `t/*.t` suite via `prove`.
//!
//! By the time this integration test runs, Cargo has already built the
//! `cdylib` target, so we just stage the artifact into `blib/` and invoke
//! `prove`. This makes `cargo test --workspace` exercise the actual
//! XSLoader-based loading path.
//!
//! Skipped automatically if `prove` is not on PATH.

use std::env;
use std::path::PathBuf;
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
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let so_src = workspace_root
        .join("target")
        .join(profile)
        .join(format!("lib{MOD_NAME}.so"));

    assert!(
        so_src.exists(),
        "cdylib not built at {} — `cargo test` should have produced it",
        so_src.display()
    );

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
