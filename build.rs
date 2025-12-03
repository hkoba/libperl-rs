use libperl_config::*;

fn main() {
    let perl = PerlConfig::default();

    // This is only needed when building a library crate (cdylib/staticlib).
    // For binary crates, you can omit this line.
    perl.emit_cargo_ldopts();

    perl.emit_features(&["useithreads"]);

    perl.emit_perlapi_vers(10, 30);
}
