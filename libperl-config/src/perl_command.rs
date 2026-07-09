use super::process_util::*;

use regex::Regex;

pub struct PerlCommand {
    perl: String,    
}

impl Default for PerlCommand {
    /// Uses the perl named by the `PERL` environment variable when set
    /// (and non-empty), falling back to `perl` on `PATH`. Build tools
    /// like ExtUtils::MakeMaker postambles pass `PERL=$(FULLPERL)` so
    /// that the perl running Makefile.PL and the perl being linked
    /// against are the same.
    fn default() -> Self {
        // Instruct cargo to re-run the calling build script when the
        // selected perl changes. Safe here: this crate is only used
        // from build scripts.
        println!("cargo:rerun-if-env-changed=PERL");
        Self {
            perl: std::env::var("PERL")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| String::from("perl")),
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
                        // Embed rpath into ALL link products of the calling
                        // crate — `cargo:rustc-link-arg=` covers cdylibs,
                        // bins, examples, AND `cargo test` binaries. The
                        // earlier `cargo:rustc-cdylib-link-arg=` form only
                        // covered cdylibs, so test binaries on perls
                        // installed in non-default locations (e.g. via
                        // `shogo82148/actions-setup-perl@v1`) failed at
                        // runtime with `libperl.so: cannot open shared
                        // object file`.
                        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libpath);
                    }
            }
            else if opt.starts_with("-l") {
                println!("cargo:rustc-link-lib={}", opt.get(2..).unwrap());
            }
        }
    }
}
