use libperl_config::*;

fn main() {
    let config = PerlConfig::default();
    config.emit_features(&["useithreads"]);
    config.emit_all_perlapi_versions(10);
}
