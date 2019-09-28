pub mod process_util {
    pub use std::process::{Command, Output, ExitStatus};
    pub use std::io::{Error, ErrorKind};
    pub use std::result::Result;
    pub use std::path::Path;

    pub fn make_command(cmd_name: &str, args: &[&str]) -> Command {
        let mut cmd = Command::new(cmd_name);
        for cf in args.iter() {
            cmd.arg(cf);
        }
        cmd
    }

    pub fn process_command_output(out: Output) -> Result<String, Error> {
        let res = String::from_utf8_lossy(&out.stdout);

        if out.stderr.len() > 0 {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(Error::new(ErrorKind::Other, [err, res].join(": ")));
        }

        Ok(res.to_string())
    }

    // pub fn run_patch(dest_fn: &str, patch_fn: &str) -> ExitStatus {
    //     Command::new("patch")
    //         .arg("-t")
    //         .arg(dest_fn)
    //         .arg(patch_fn)
    //         .status()
    //         .unwrap()
    // }
}

pub mod perl_config {
    use super::process_util::*;

    use regex::Regex;
    // use std::collections::HashMap;

    pub fn perl_embed_ccopts() -> Result<Vec<String>, Error> {
        perl_embed_opts("ccopts", r"^-[ID]")
    }

    pub fn perl_embed_ldopts() -> Result<Vec<String>, Error> {
        perl_embed_opts("ldopts", r"^-[lL]")
    }

    pub fn perl_embed_opts(cmd: &str, prefix: &str) -> Result<Vec<String>, Error> {
        let mut cmd = make_command("perl", &["-MExtUtils::Embed", "-e", cmd]);

        let out_str = process_command_output(cmd.output()?)?;

        let re = Regex::new(prefix).unwrap();
        Ok(out_str
            .split_whitespace()
            .map(String::from)
            .filter(|s| re.is_match(s))
            .collect())
    }
    
    pub fn emit_cargo_ldopts() {
        let ldopts = perl_embed_ldopts().unwrap();
        println!("# perl ldopts = {:?}, ", ldopts);

        for opt in perl_embed_ldopts().unwrap().iter() {
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

fn main() {
    perl_config::emit_cargo_ldopts();
}
