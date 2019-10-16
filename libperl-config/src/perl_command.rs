use super::process_util::*;

use regex::Regex;

pub struct PerlCommand {
    perl: String,    
}

impl Default for PerlCommand {
    fn default() -> Self {
        Self {
            perl: String::from("perl")
        }
    }
}

impl PerlCommand {

    pub fn new(perl: &str) -> Self {
        Self {
            perl: String::from(perl),
        }
    }

    pub fn command(&self, args: &[&str]) -> Command {
        make_command(self.perl.as_str(), args)
    }

    pub fn read_raw_config(&self, configs: &[&str]) -> Result<String, Error> {
        let script = ["-wle", r#"
    use strict;
    use Config;
    print join "\t", $_, ($Config{$_} // '')
      for @ARGV ? @ARGV : sort keys %Config;
    "#
        ];
        let mut cmd = self.command(&[&script[..], configs].concat());
        
        process_command_output(cmd.output()?)
    }

    pub fn read_ccopts(&self) -> Result<Vec<String>, Error> {
        self.read_embed_opts("ccopts", r"^-[ID]")
    }

    pub fn read_ldopts(&self) -> Result<Vec<String>, Error> {
        self.read_embed_opts("ldopts", r"^-[lL]")
    }

    pub fn read_raw_embed_opts(&self, cmd: &str) -> Result<String, Error> {
        let mut cmd = self.command(
            &["-MExtUtils::Embed", "-e", cmd],
        );

        process_command_output(cmd.output()?)
    }

    pub fn read_embed_opts(&self, cmd: &str, prefix: &str) -> Result<Vec<String>, Error> {
        let out_str = self.read_raw_embed_opts(cmd)?;

        let re = Regex::new(prefix).unwrap();
        Ok(out_str
           .split_whitespace()
           .map(String::from)
           .filter(|s| re.is_match(s))
           .collect())
    }

    pub fn emit_cargo_ldopts(&self) {
        let ldopts = self.read_ldopts().unwrap();
        println!("# perl ldopts = {:?}, ", ldopts);

        for opt in self.read_ldopts().unwrap().iter() {
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
