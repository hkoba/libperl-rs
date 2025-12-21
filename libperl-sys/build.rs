extern crate bindgen;

use libperl_config::*;

use std::env;
use std::io::Write;
use std::path::{PathBuf, Path};

use quote::ToTokens;
use syn::{FnArg, ForeignItem, Item, Pat, ReturnType};

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

    let archlib = String::from(&perl.dict["archlib"]);
    let perl_h = Path::new(&archlib).join("CORE/perl.h");
    let cop_h = Path::new(&archlib).join("CORE/cop.h");

    let ccopts = perl.read_ccopts().unwrap();
    println!("# perl ccopts = {:?}, ", ccopts);

    perl.emit_features(&["useithreads"]); // "usemultiplicity"

    perl.emit_perlapi_vers(10, 40);

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

            .derive_debug(true)
            .impl_debug(true)
            .formatter(bindgen::Formatter::Prettyplease)
            .rustified_enum("OPclass|opcode|svtype")

        // The input header we would like to generate
        // bindings for.
            .header(src_file_name)

            .clang_arg("-DPERL_CORE")
            .clang_args(ccopts.iter())

            .opaque_type("timex")

            .allowlist_file(perl_h.to_str().unwrap())
            .allowlist_file(cop_h.to_str().unwrap())
            .allowlist_item("opcode")
            .allowlist_item("(Perl|perl|PL)_.*")
            .allowlist_item("([SAHRGC]V|xpv).*")
            .allowlist_item("OP.*")
            .allowlist_item("G_.*")

        // Finish the builder and generate the bindings.
            .generate()
        // Unwrap the Result and panic on failure.
            .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
        bindings
            .write_to_file(out_file.to_str().unwrap())
            .expect("Couldn't write bindings!");

    }

    // Generate sigdb.rs from bindings.rs
    generate_sigdb(&out_file, &cargo_outdir().join("sigdb.rs"));
}

fn return_type_to_string(ret: &ReturnType) -> String {
    match ret {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, ty) => ty.to_token_stream().to_string(),
    }
}

fn extract_args(inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>) -> Vec<(String, String)> {
    inputs
        .iter()
        .map(|arg| {
            match arg {
                FnArg::Typed(pat_type) => {
                    let name = match &*pat_type.pat {
                        Pat::Ident(ident) => ident.ident.to_string(),
                        Pat::Wild(_) => String::new(),
                        _ => String::new(),
                    };
                    let ty = pat_type.ty.to_token_stream().to_string();
                    (name, ty)
                }
                FnArg::Receiver(_) => ("self".to_string(), "Self".to_string()),
            }
        })
        .collect()
}

fn generate_sigdb(bindings_path: &Path, sigdb_path: &Path) {
    let content = std::fs::read_to_string(bindings_path)
        .expect("Failed to read bindings.rs");
    let syntax = syn::parse_file(&content)
        .expect("Failed to parse bindings.rs");

    let mut funcs: Vec<(String, String, Vec<(String, String)>, bool)> = Vec::new();

    for item in &syntax.items {
        if let Item::ForeignMod(foreign) = item {
            for foreign_item in &foreign.items {
                if let ForeignItem::Fn(f) = foreign_item {
                    let name = f.sig.ident.to_string();
                    let ret = return_type_to_string(&f.sig.output);
                    let args = extract_args(&f.sig.inputs);
                    let is_variadic = f.sig.variadic.is_some();
                    funcs.push((name, ret, args, is_variadic));
                }
            }
        }
    }

    // Build sigdb.rs
    let mut out = std::fs::File::create(sigdb_path)
        .expect("Failed to create sigdb.rs");

    writeln!(out, "// This file is auto-generated by build.rs. Do not edit.").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "use phf::phf_map;").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "#[derive(Debug, Clone, Copy)]").unwrap();
    writeln!(out, "pub struct FnId(pub u32);").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "#[derive(Debug, Clone)]").unwrap();
    writeln!(out, "pub struct FnSig {{").unwrap();
    writeln!(out, "    pub name: &'static str,").unwrap();
    writeln!(out, "    pub ret: &'static str,").unwrap();
    writeln!(out, "    pub args: &'static [(&'static str, &'static str)],").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // Generate FUNCS array
    writeln!(out, "pub static FUNCS: &[FnSig] = &[").unwrap();
    for (name, ret, args, is_variadic) in &funcs {
        writeln!(out, "    FnSig {{").unwrap();
        writeln!(out, "        name: {:?},", name).unwrap();
        writeln!(out, "        ret: {:?},", ret).unwrap();
        write!(out, "        args: &[").unwrap();
        for (arg_name, arg_ty) in args {
            write!(out, "({:?}, {:?}), ", arg_name, arg_ty).unwrap();
        }
        if *is_variadic {
            write!(out, "(\"...\", \"\"), ").unwrap();
        }
        writeln!(out, "],").unwrap();
        writeln!(out, "    }},").unwrap();
    }
    writeln!(out, "];").unwrap();
    writeln!(out).unwrap();

    // Generate FN_BY_NAME map using phf_map!
    writeln!(out, "pub static FN_BY_NAME: phf::Map<&'static str, FnId> = phf_map! {{").unwrap();
    for (idx, (name, _, _, _)) in funcs.iter().enumerate() {
        writeln!(out, "    {:?} => FnId({}),", name, idx).unwrap();
    }
    writeln!(out, "}};").unwrap();

    println!("# Generated sigdb.rs with {} functions", funcs.len());
}
