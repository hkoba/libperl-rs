use libperl_config;

fn main() {
    let cfg = libperl_config::PerlConfig::default();
    cfg.emit_cargo_ldopts();
}
