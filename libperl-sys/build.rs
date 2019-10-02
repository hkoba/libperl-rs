extern crate bindgen;

use libperl_config::*;

use std::env;
use std::path::{PathBuf, Path};

fn is_older_file(dest: &Path, src: &Path) -> bool {
    dest.metadata().unwrap().modified().unwrap()
        < src.metadata().unwrap().modified().unwrap()
}

fn main() {

    let perl = PerlConfig::default();
    perl.emit_cargo_ldopts();

    let ccopts = perl.read_ccopts().unwrap();
    println!("# perl ccopts = {:?}, ", ccopts);

    let configs = ["useithreads"];
    let dict = perl.read_config(&configs).unwrap();

    for &cfg in configs.iter() {
        println!("# perl config {} = {:?}", cfg, dict.get(&String::from(cfg)));
        if PerlConfig::is_defined_in(&dict, cfg).unwrap() {
            println!("cargo:rustc-cfg=perl_{}", cfg);
        }
    }

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
