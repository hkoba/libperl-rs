
mod perl_config;
pub use perl_config::*;

pub mod process_util;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let cfg = super::PerlConfig::default();
        assert!(cfg.read_ccopts().unwrap().len() > 0);
        assert!(cfg.read_ldopts().unwrap().len() > 0);
    }
    
    #[test]
    fn can_read_config() {
        let cfg = super::PerlConfig::default();
        let dict = cfg.read_config(&[]).unwrap();
        let perl_version = dict.get("PERL_VERSION");
        assert_ne!(perl_version, None);
        if let Some(ver) = perl_version {
            let script = r#"
use strict;
use Config;
print "PERL_VERSION\t", $Config{PERL_VERSION};
"#;
            assert_eq!(super::process_util::process_command_output(
                cfg.command(&["-e", script]).output().unwrap()
            ).unwrap(), ["PERL_VERSION", ver].join("\t"))
        }
    }
}
