use libperl_config::*;

fn main() {
    let config = PerlConfig::default();
    // Embed rpath / link-search for libperl.so so that `cargo test`'s
    // unittests binary can find it at runtime — matters on perls
    // installed in non-default locations (e.g. plenv or
    // `shogo82148/actions-setup-perl@v1`). `cargo:rustc-link-arg=` does
    // not propagate from libperl-sys via cargo's dependency graph, so
    // each crate that produces a binary linking libperl must call this.
    config.emit_cargo_ldopts();
    config.emit_features(&["useithreads"]);
    config.emit_all_perlapi_versions(10);
}
