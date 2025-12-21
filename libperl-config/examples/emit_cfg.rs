use libperl_config::*;

fn main() {
    let config = PerlConfig::default();
    config.emit_all_perlapi_versions(10);
    // config.emit_perlapi_vers(10, 40);
}
