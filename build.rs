use libperl_config::*;

fn main() {
    let config = PerlConfig::default();

    // This is only needed when building a library crate (cdylib/staticlib).
    // For binary crates, you can omit this line.
    config.emit_cargo_ldopts();

    config.emit_features(&["useithreads"]);

    config.emit_perlapi_vers(10, config.perl_api_version());
}
