extern crate bindgen;

use std::env;
use std::path::{PathBuf, Path};

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

fn is_older_file(dest: &Path, src: &Path) -> bool {
    dest.metadata().unwrap().modified().unwrap()
        < src.metadata().unwrap().modified().unwrap()
}

fn main() {

    perl_config::emit_cargo_ldopts();

    let ccopts = perl_config::perl_embed_ccopts().unwrap();
    println!("# perl ccopts = {:?}, ", ccopts);

    let src_file_name = "wrapper.h";
    let src_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join(src_file_name);

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file = out_path.join("bindings.rs");

    let do_build = if !out_file.exists() {
        println!("# will generate new {}", out_file.display());
        true
    } else if is_older_file(&out_file, &src_path) {
        println!("# out_file {} is older than src {}"
                 , out_file.display(), src_path.display());
        true
    } else {
        println!("# out_file {} exists and up-to-date with src {}\n# out_file={{{:?}}} src_file={{{:?}}}"
                 , out_file.display(), src_path.display()
                 , out_file.metadata().unwrap().modified()
                 , src_path.metadata().unwrap().modified()
        );
        false
    };

    if do_build {
        // The bindgen::Builder is the main entry point
        // to bindgen, and lets you build up options for
        // the resulting bindings.
        let bindings = bindgen::Builder::default()

            .rustfmt_bindings(true)

        // The input header we would like to generate
        // bindings for.
            .header(src_file_name)

            .clang_arg("-DPERL_CORE")
            .clang_args(ccopts.iter())

            .opaque_type("timex")

            .blacklist_item("IPPORT_RESERVED")

            .blacklist_item("FP_.*")
        // .blacklist_item("FP_INT_UPWARD")
        // .blacklist_item("FP_INT_DOWNWARD")
        // .blacklist_item("FP_INT_TOWARDZERO")
        // .blacklist_item("FP_INT_TONEARESTFROMZERO")
        // .blacklist_item("FP_INT_TONEAREST")
        // .blacklist_item("FP_NAN")
        // .blacklist_item("FP_INFINITE")
        // .blacklist_item("FP_ZERO")
        // .blacklist_item("FP_SUBNORMAL")
        // .blacklist_item("FP_NORMAL")

            .blacklist_function("f?printf")
            .blacklist_function("f?scanf")

        // Finish the builder and generate the bindings.
            .generate()
        // Unwrap the Result and panic on failure.
            .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
        bindings
            .write_to_file(out_file.to_str().unwrap())
            .expect("Couldn't write bindings!");

    }
}
