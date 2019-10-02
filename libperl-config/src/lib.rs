
mod perl_config;
pub use perl_config::*;

pub mod process_util;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let cfg = super::PerlConfig::default();
        assert!(cfg.embed_ccopts().unwrap().len() > 0);
        assert!(cfg.embed_ldopts().unwrap().len() > 0);
    }
}
