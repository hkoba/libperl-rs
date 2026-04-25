extern crate bindgen;

use libperl_config::*;
use libperl_macrogen::Pipeline;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{PathBuf, Path};
use std::process::Command;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let perl = PerlConfig::default();
    perl.emit_cargo_ldopts();

    let archlib = String::from(&perl.dict["archlib"]);
    let perl_h = Path::new(&archlib).join("CORE/perl.h");
    let cop_h = Path::new(&archlib).join("CORE/cop.h");

    let ccopts = perl.read_ccopts().unwrap();
    println!("# perl ccopts = {:?}, ", ccopts);

    perl.emit_features(&["useithreads"]); // "usemultiplicity"

    perl.emit_all_perlapi_versions(10);

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
            .rustified_enum(".*") // every enum

            .derive_partialeq(true)   // #[derive(PartialEq)]
            .derive_eq(true)          // #[derive(Eq)]
            .derive_partialord(true)  // #[derive(PartialOrd)]
            .derive_ord(true)         // #[derive(Ord)]
            // .flexarray_dst(true)      // flexible array members

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
            .allowlist_item("regex_charset")
            .allowlist_item("SCX_enum")

        // Finish the builder and generate the bindings.
            .generate()
        // Unwrap the Result and panic on failure.
            .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
        bindings
            .write_to_file(out_file.to_str().unwrap())
            .expect("Couldn't write bindings!");

        let macro_out_path = cargo_outdir().join("macro_bindings.rs");
        let mut output = File::create(&macro_out_path)?;

        let mut builder = Pipeline::builder("xs-wrapper.h")
            .with_auto_perl_config()?
            .with_bindings(&out_file)
            .with_codegen_defaults();

        let skip_list = cargo_topdir_file("skip-codegen.txt");

        if skip_list.exists() {
            builder = builder.with_skip_codegen_list(&skip_list);
            println!("cargo:rerurn-if-changed={}", skip_list.display());
        }

        for p in cc_system_includes() {
            builder = builder.with_include(p);
        }

        let _result = builder
            .build()?
            .generate(&mut output)?;

        // println!("cargo:warning=Generated {} macro + {} inline functions",
        //          result.stats.macro_success, result.stats.inline_success);

    }

    // Generate sigdb.rs from bindings.rs
    generate_sigdb(&out_file, &cargo_outdir().join("sigdb.rs"));

    Ok(())
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

/// Discover the actual system include paths from the running C compiler.
/// Used to bridge the gap between Perl's recorded `incpth` (which can point
/// to a gcc version not present on the host, e.g. on GitHub Actions) and
/// the headers actually available on the runner.
fn cc_system_includes() -> Vec<PathBuf> {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let output = match Command::new(&cc)
        .args(["-E", "-Wp,-v", "-xc", "/dev/null"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            println!("cargo:warning=cc_system_includes: failed to run {}: {}", cc, e);
            return Vec::new();
        }
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut paths = Vec::new();
    let mut in_list = false;
    for line in stderr.lines() {
        if line.contains("#include <...> search starts here") {
            in_list = true;
            continue;
        }
        if line.contains("End of search list") {
            break;
        }
        if in_list {
            let p = PathBuf::from(line.trim());
            if p.is_dir() {
                paths.push(p);
            }
        }
    }
    paths
}
