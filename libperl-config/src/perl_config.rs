use super::process_util::*;

use regex::Regex;

pub struct PerlConfig {
    perl: String,
}

impl Default for PerlConfig {
    fn default() -> Self {
        Self {
            perl: String::from("perl")
        }
    }
}

impl PerlConfig {

    pub fn new(perl: &str) -> Self {
        Self {
            perl: String::from(perl),
        }
    }

    pub fn embed_ccopts(&self) -> Result<Vec<String>, Error> {
        self.embed_opts("ccopts", r"^-[ID]")
    }

    pub fn embed_ldopts(&self) -> Result<Vec<String>, Error> {
        self.embed_opts("ldopts", r"^-[lL]")
    }

    pub fn embed_opts(&self, cmd: &str, prefix: &str) -> Result<Vec<String>, Error> {
        let mut cmd = make_command(
            self.perl.as_str(),
            &["-MExtUtils::Embed", "-e", cmd],
        );

        let out_str = process_command_output(cmd.output()?)?;

        let re = Regex::new(prefix).unwrap();
        Ok(out_str
           .split_whitespace()
           .map(String::from)
           .filter(|s| re.is_match(s))
           .collect())
    }

    pub fn emit_cargo_ldopts(&self) {
        let ldopts = self.embed_ldopts().unwrap();
        println!("# perl ldopts = {:?}, ", ldopts);

        for opt in self.embed_ldopts().unwrap().iter() {
            if opt.starts_with("-L") {
                let libpath = opt.get(2..).unwrap();
                println!("cargo:rustc-link-search={}", libpath);
                if std::path::Path::new(libpath).file_name()
                    == Some(std::ffi::OsStr::new("CORE")) {
                        println!("cargo:rustc-cdylib-link-arg=-Wl,-rpath,{}", libpath);
                    }
            }
            else if opt.starts_with("-l") {
                println!("cargo:rustc-link-lib={}", opt.get(2..).unwrap());
            }
        }
    }
}
