use libperl_config::*;

fn main() {
    let perl = PerlConfig::default();
    perl.emit_cargo_ldopts();

    perl.emit_features(&["useithreads"]);
}
