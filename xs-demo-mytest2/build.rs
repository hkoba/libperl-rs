use libperl_config::*;

fn main() {
    let cfg = PerlConfig::default();
    cfg.emit_cargo_ldopts();
    cfg.emit_features(&["useithreads"]);
}
