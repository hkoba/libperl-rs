extern crate bindgen;

use std::env;
use std::path::PathBuf;

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

    pub fn run_patch(dest_fn: &str, patch_fn: &str) -> ExitStatus {
        Command::new("patch")
            .arg("-t")
            .arg(dest_fn)
            .arg(patch_fn)
            .status()
            .unwrap()
    }
}

pub mod perl_config {
    use super::process_util::*;

    use regex::Regex;
    // use std::collections::HashMap;

    pub fn perl_embed_opts() -> Result<Vec<String>, Error> {
        let mut cmd = make_command("perl", &["-MExtUtils::Embed", "-e", "ccopts"]);

        let out_str = process_command_output(cmd.output()?)?;

        let re = Regex::new(r"^-[ID]").unwrap();
        Ok(out_str
            .split_whitespace()
            .map(String::from)
            .filter(|s| re.is_match(s))
            .collect())
    }
}


fn main() {
    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=perl");

    let emb_opts = perl_config::perl_embed_opts().unwrap();

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .clang_arg("-DPERL_CORE")
        .clang_args(emb_opts.iter())
        .opaque_type("timex")

        .blacklist_item("IPPORT_RESERVED")

        // .blacklist_item("^FP_") // Not worked.
        .blacklist_item("FP_INT_UPWARD")
        .blacklist_item("FP_INT_DOWNWARD")
        .blacklist_item("FP_INT_TOWARDZERO")
        .blacklist_item("FP_INT_TONEARESTFROMZERO")
        .blacklist_item("FP_INT_TONEAREST")
        .blacklist_item("FP_NAN")
        .blacklist_item("FP_INFINITE")
        .blacklist_item("FP_ZERO")
        .blacklist_item("FP_SUBNORMAL")
        .blacklist_item("FP_NORMAL")

        .blacklist_function("fprintf")
        .blacklist_function("printf")
        .blacklist_function("fscanf")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file = out_path.join("bindings.rs");
    bindings
        .write_to_file(out_file.to_str().unwrap())
        .expect("Couldn't write bindings!");

    // let topdir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // let rc = process_util::run_patch(
    //     out_file.to_str().unwrap(),
    //     topdir.join("src/bindings.patch").to_str().unwrap()
    // );
    // if ! rc.success() {
    //     panic!("patch failed")
    // }
}
