//! Cargo-test bridge: run the Perl-side `t/*.t` suite via `prove`.
//! See `xs-demo-mytest/tests/perl_smoke.rs` for the design rationale.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const MOD_NAME: &str = "Mytest2";

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

fn build_cdylib(workspace_root: &Path) -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let so = workspace_root
        .join("target")
        .join(profile)
        .join(format!("lib{MOD_NAME}.so"));
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
    assert!(so.exists(), "cdylib not produced by cargo build at {}", so.display());
    so
}
