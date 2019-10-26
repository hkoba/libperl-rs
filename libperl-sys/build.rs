extern crate bindgen;

use libperl_config::*;

use std::env;
use std::path::{PathBuf, Path};

fn is_older_file(dest: &Path, src: &Path) -> bool {
    dest.metadata().unwrap().modified().unwrap()
        < src.metadata().unwrap().modified().unwrap()
}

fn cargo_topdir_file(file: &str) -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join(file)
}

fn cargo_outdir() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

fn look_updated_against<'a>(checked: &Path, against: &[&'a Path]) -> Option<&'a Path> {
    for f in against.iter() {
        if is_older_file(checked, f) {
            return Some(f)
        }
    }
    None
}

fn main() {

    let perl = PerlConfig::default();
    perl.emit_cargo_ldopts();

    let ccopts = perl.read_ccopts().unwrap();
    println!("# perl ccopts = {:?}, ", ccopts);

    perl.emit_features(&["useithreads"]); // "usemultiplicity"

    perl.emit_perlapi_vers(10, 30);

    let src_file_name = "wrapper.h";
    let src_path = cargo_topdir_file(src_file_name);

    let out_file = cargo_outdir().join("bindings.rs");

    let do_build = if !out_file.exists() {
        println!("# will generate new {}", out_file.display());
        true
    }
    else if let Some(src_path) = look_updated_against(
        &out_file, &[
            &src_path,
            &cargo_topdir_file("build.rs"),
        ]) {
        println!("# out_file {} is older than src {}"
                 , out_file.display(), src_path.display());
        true
    }
    else {
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
            .derive_debug(true)
            .impl_debug(true)

            .rustified_enum("OPclass|opcode")

        // The input header we would like to generate
        // bindings for.
            .header(src_file_name)

            .clang_arg("-DPERL_CORE")
            .clang_args(ccopts.iter())

            .opaque_type("timex")

            .blacklist_type("max_align_t")

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
