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

    // GH-21 で判明した第 2 の原因: libperl-macrogen (build.rs 内で
    // インプロセス実行) は Config.pm を PATH 上の `perl` から読むため、
    // PERL=... で選んだ perl と食い違うと bindgen 出力 (PERL 準拠) と
    // macrogen 出力 (PATH 準拠) が別世代の混成になる。選択した perl の
    // bin を自プロセスの PATH 先頭に足し、子プロセスの `perl` 解決を
    // PERL と一致させる。
    if let Some(perl_path) = env::var("PERL").ok().filter(|s| !s.is_empty()) {
        let abs = std::fs::canonicalize(&perl_path)
            .unwrap_or_else(|_| PathBuf::from(&perl_path));
        if let Some(bin_dir) = abs.parent().filter(|d| d.as_os_str() != "") {
            let path_var = env::var("PATH").unwrap_or_default();
            // SAFETY: build script 冒頭・単一スレッドの時点で呼ぶ
            unsafe {
                env::set_var(
                    "PATH",
                    format!("{}:{}", bin_dir.display(), path_var),
                );
            }
        }
    }

    let perl = PerlConfig::default();
    perl.emit_cargo_ldopts();

    let archlib = String::from(&perl.dict["archlib"]);
    let perl_h = Path::new(&archlib).join("CORE/perl.h");
    let cop_h = Path::new(&archlib).join("CORE/cop.h");

    let ccopts = perl.read_ccopts().unwrap();
    println!("# perl ccopts = {:?}, ", ccopts);

    perl.emit_features(&["useithreads"]); // "usemultiplicity"

    perl.emit_all_perlapi_versions(10);

    // Surface the build-target Perl version into the rlib so that
    // `env!()` in lib.rs can interpolate it into the crate-level
    // doc comment (visible on docs.rs) and into `pub const`s
    // (queryable at runtime). Crucial for docs.rs visitors who need
    // to know which Perl version's API surface they're looking at.
    let perl_version  = perl.dict.get("version").cloned().unwrap_or_default();
    let perl_archname = perl.dict.get("archname").cloned().unwrap_or_default();
    let perl_use_ithreads = perl.is_defined("useithreads").unwrap_or(false);
    let perl_threaded = if perl_use_ithreads {
        "threaded"
    } else {
        "non-threaded"
    };
    println!("cargo:rustc-env=LIBPERL_SYS_PERL_VERSION={}", perl_version);
    println!("cargo:rustc-env=LIBPERL_SYS_PERL_ARCHNAME={}", perl_archname);
    println!("cargo:rustc-env=LIBPERL_SYS_PERL_THREADED={}", perl_threaded);

    let src_file_name = "wrapper.h";
    let src_path = cargo_topdir_file(src_file_name);
    let build_rs_path = cargo_topdir_file("build.rs");

    // skip-codegen.txt は do_build の鮮度判定と rerun 監視の両方に入れる。
    // 登録を do_build ブロック内に置くと、do_build=false で終わった run の
    // 後は cargo が監視をやめ、リスト編集が二度と反映されなくなる。
    let skip_list = cargo_topdir_file("skip-codegen.txt");
    if skip_list.exists() {
        println!("cargo:rerun-if-changed={}", skip_list.display());
    }

    let perl_minor: u32 = {
        let mut it = perl_version.split('.');
        let major: u32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor: u32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        if major == 5 { minor } else { 0 }
    };

    // perl < 5.38 では apidoc の型宣言が足りず、macrogen が生成する
    // マクロの一部が誤型で出る (GH-16)。旧バージョンだけ追加の skip
    // リストを適用する。
    let legacy_skip_list = cargo_topdir_file("skip-codegen-legacy.txt");
    let use_legacy_skip = perl_minor < 38 && legacy_skip_list.exists();
    if use_legacy_skip {
        println!("cargo:rerun-if-changed={}", legacy_skip_list.display());
    }

    // 5.32〜5.40 でのみ生成が不成立になる関数向け (< 5.42 で適用)。
    // 詳細は skip-codegen-pre42.txt のコメント参照。
    let pre42_skip_list = cargo_topdir_file("skip-codegen-pre42.txt");
    let use_pre42_skip = perl_minor < 42 && pre42_skip_list.exists();
    if use_pre42_skip {
        println!("cargo:rerun-if-changed={}", pre42_skip_list.display());
    }

    // Partial eval 必須 API (GH-16)。生成されなかったら理由付きで fail-fast。
    let require_list = cargo_topdir_file("require-codegen.txt");
    if require_list.exists() {
        println!("cargo:rerun-if-changed={}", require_list.display());
    }

    // threaded 限定の必須 API (PAD_SET_CUR は non-threaded では
    // PAD_SET_CUR_NOSAVE が unresolved で生成されないため別リスト)。
    let threaded_require_list = cargo_topdir_file("require-codegen-threaded.txt");
    let use_threaded_require = perl_use_ithreads && threaded_require_list.exists();
    if use_threaded_require {
        println!("cargo:rerun-if-changed={}", threaded_require_list.display());
    }

    let out_file = cargo_outdir().join("bindings.rs");

    // mtime 比較は wrapper.h / build.rs / skip リストの変化しか見ないため、
    // macrogen 側だけが更新された場合 (dependency bump や path 依存での開発中)
    // に stale な macro_bindings.rs が残る (doc/notes-macrogen-0.1.8-integration.md
    // §4)。macrogen の apidoc data version をスタンプファイルに書き、
    // 現在の値と食い違ったら再生成を強制する。
    let macrogen_stamp = cargo_outdir().join("macrogen-apidoc-version.txt");
    let macrogen_updated = match std::fs::read_to_string(&macrogen_stamp) {
        Ok(s) => s.trim() != libperl_macrogen::apidoc_data::APIDOC_DATA_VERSION,
        Err(_) => true,
    };

    // GH-21: mtime 比較も macrogen stamp も「どの perl から生成したか」を
    // 見ていないため、perl を切り替えて再実行すると (Makefile.PL の
    // PERL=... 再指定など)、build.rs 自体は再走するのに bindings.rs /
    // macro_bindings.rs が前の perl のまま残る — 前の版の bindings が
    // 新しい版の cfg(perlapi_ver*) でコンパイルされる。生成元 perl の
    // 同一性をスタンプに書き、現在の perl と食い違ったら再生成を強制する。
    let perl_identity = format!(
        "{} {} ithreads={} shrplib={} archlib={}",
        perl_version,
        perl_archname,
        perl_use_ithreads,
        perl.dict.get("useshrplib").cloned().unwrap_or_default(),
        archlib,
    );
    let perl_stamp = cargo_outdir().join("perl-identity.txt");
    let perl_switched = match std::fs::read_to_string(&perl_stamp) {
        Ok(s) => s.trim() != perl_identity,
        Err(_) => true,
    };

    let mut freshness_deps: Vec<&Path> = vec![&src_path, &build_rs_path];
    if skip_list.exists() {
        freshness_deps.push(&skip_list);
    }
    if use_legacy_skip {
        freshness_deps.push(&legacy_skip_list);
    }
    if use_pre42_skip {
        freshness_deps.push(&pre42_skip_list);
    }
    if require_list.exists() {
        freshness_deps.push(&require_list);
    }
    if use_threaded_require {
        freshness_deps.push(&threaded_require_list);
    }

    // docs.rs's build sandbox sometimes presents an OUT_DIR with a
    // pre-existing `bindings.rs`, even though the per-version
    // `target/` host path implies a fresh start. Whatever the root
    // cause (rustwide caching, snapshot reuse, ...), the `do_build`
    // short-circuit below would otherwise kick in and skip both
    // bindgen AND macrogen — leaving the crate documented with
    // stale generated sources. Always force a rebuild in DOCS_RS
    // mode. Locally, the existing freshness check still applies
    // (cargo's own incremental tracking handles the common case).
    let force_rebuild = env::var("DOCS_RS").is_ok();
    let do_build = if force_rebuild || !out_file.exists() {
        if !force_rebuild {
            println!("# will generate new {}", out_file.display());
        }
        true
    }
    else if perl_switched {
        println!("# generating perl changed (stamp {} != {}), regenerating"
                 , std::fs::read_to_string(&perl_stamp)
                     .map(|s| s.trim().to_string())
                     .unwrap_or_else(|_| "<missing>".to_string())
                 , perl_identity);
        true
    }
    else if macrogen_updated {
        println!("# macrogen apidoc data version changed (stamp {} != {}), regenerating"
                 , std::fs::read_to_string(&macrogen_stamp)
                     .map(|s| s.trim().to_string())
                     .unwrap_or_else(|_| "<missing>".to_string())
                 , libperl_macrogen::apidoc_data::APIDOC_DATA_VERSION);
        true
    }
    else if let Some(src_path) = look_updated_against(
        &out_file, &freshness_deps) {
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
            // OP_ARGCHECK (signature) の aux 構造体と、
            // OP_MULTIDEREF の aux アクション定数 (静的解析ツール向け)
            .allowlist_item("op_argcheck_aux")
            .allowlist_item("MDEREF_.*")

        // Finish the builder and generate the bindings.
            .generate()
        // Unwrap the Result and panic on failure.
            .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
        bindings
            .write_to_file(out_file.to_str().unwrap())
            .expect("Couldn't write bindings!");

        patch_unsized_arrays(&out_file, &archlib);

        let macro_out_path = cargo_outdir().join("macro_bindings.rs");
        let mut output = File::create(&macro_out_path)?;

        let mut builder = Pipeline::builder("xs-wrapper.h")
            .with_auto_perl_config()?
            .with_bindings(&out_file)
            .with_codegen_defaults();

        if skip_list.exists() {
            builder = builder.with_skip_codegen_list(&skip_list);
        }
        if use_legacy_skip {
            builder = builder.with_skip_codegen_list(&legacy_skip_list);
        }
        if use_pre42_skip {
            builder = builder.with_skip_codegen_list(&pre42_skip_list);
        }
        if require_list.exists() {
            builder = builder.with_require_codegen_list(&require_list);
        }
        if use_threaded_require {
            builder = builder.with_require_codegen_list(&threaded_require_list);
        }

        for p in cc_system_includes() {
            builder = builder.with_include(p);
        }

        let generated = builder
            .build()?
            .generate(&mut output)?;

        // GH-20 (a): read 専用の PL_xxx! と同じ情報源 (PERLVAR 観測) から
        // 読み書き両用の PL_xxx_ptr! を生成する
        emit_perlvar_ptr_macros(
            &cargo_outdir().join("perlvar_ptr_bindings.rs"),
            &generated.result().perlvar_dict,
            generated.result().perl_build_mode.is_threaded(),
        )?;

        std::fs::write(&macrogen_stamp,
                       libperl_macrogen::apidoc_data::APIDOC_DATA_VERSION)?;
        std::fs::write(&perl_stamp, &perl_identity)?;
    }

    // Generate sigdb.rs from bindings.rs
    let extern_fns = collect_extern_fns(&out_file);
    generate_sigdb(&extern_fns, &cargo_outdir().join("sigdb.rs"));

    // GH-20 (b): threaded 風呼び出し規約の shim。bindgen の extern に加え、
    // macrogen 生成のインライン関数 (macro_bindings.rs) もモード依存の
    // シグネチャを持つため対象に含める。
    let mut thx_fns = extern_fns;
    thx_fns.extend(collect_pub_fns(&cargo_outdir().join("macro_bindings.rs")));
    generate_thx(&thx_fns, &cargo_outdir().join("thx_bindings.rs"));

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

/// (関数名, 返り値型, 引数 (名前, 型) 列, variadic か)
type FnDecl = (String, String, Vec<(String, String)>, bool);

/// bindings.rs の extern "C" 宣言を集める (sigdb / thx の共通入力)。
fn collect_extern_fns(bindings_path: &Path) -> Vec<FnDecl> {
    let content = std::fs::read_to_string(bindings_path)
        .expect("Failed to read bindings.rs");
    let syntax = syn::parse_file(&content)
        .expect("Failed to parse bindings.rs");

    let mut funcs: Vec<FnDecl> = Vec::new();
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
    funcs
}

/// macrogen 生成物 (macro_bindings.rs) のトップレベル pub fn を集める。
/// static inline 由来の Perl_* もモード依存シグネチャを持つので、
/// thx shim はこちらも包む必要がある。
fn collect_pub_fns(path: &Path) -> Vec<FnDecl> {
    let content = std::fs::read_to_string(path)
        .expect("Failed to read macro_bindings.rs");
    let syntax = syn::parse_file(&content)
        .expect("Failed to parse macro_bindings.rs");

    let mut funcs: Vec<FnDecl> = Vec::new();
    for item in &syntax.items {
        if let Item::Fn(f) = item {
            if !matches!(f.vis, syn::Visibility::Public(_)) {
                continue;
            }
            let name = f.sig.ident.to_string();
            let ret = return_type_to_string(&f.sig.output);
            let args = extract_args(&f.sig.inputs);
            let is_variadic = f.sig.variadic.is_some();
            funcs.push((name, ret, args, is_variadic));
        }
    }
    funcs
}

fn generate_sigdb(funcs: &[FnDecl], sigdb_path: &Path) {
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
    for (name, ret, args, is_variadic) in funcs {
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

/// Patch unsized C array declarations in the bindgen output so that
/// indexed access works at runtime.
///
/// Some Perl globals are declared as unsized C arrays in the headers:
///
/// ```c
/// EXTCONST char* const PL_op_name[];   /* in opcode.h */
/// EXTCONST char* const PL_op_desc[];   /* in opcode.h */
/// EXT Perl_ppaddr_t PL_ppaddr[];       /* in opcode.h (mutable -> static mut) */
/// EXT Perl_check_t  PL_check[];        /* in opcode.h (mutable -> static mut) */
/// ```
///
/// bindgen turns these into `[T; 0usize]`, which makes any `PL_op_name[i]`
/// access panic at runtime ("index out of bounds"). C-side redeclaration
/// in `wrapper.h` does not help because bindgen keeps the size from the
/// first declaration it sees.
///
/// We post-process `bindings.rs` to replace `0usize` with the correct
/// length. The length comes from `#define MAXO N` in `opnames.h`, which
/// is exactly the number of opcodes for the target Perl.
fn patch_unsized_arrays(bindings_path: &Path, archlib: &str) {
    let opnames_h = Path::new(archlib).join("CORE/opnames.h");
    let maxo = read_define_int(&opnames_h, "MAXO").unwrap_or_else(|| {
        panic!(
            "patch_unsized_arrays: could not read `#define MAXO` from {}",
            opnames_h.display()
        )
    });

    // (symbol name, length) pairs — extend as new unsized arrays appear.
    let entries: &[(&str, usize)] = &[
        ("PL_op_name", maxo),
        ("PL_op_desc", maxo),
        ("PL_ppaddr", maxo),
        ("PL_check", maxo),
    ];

    let original = std::fs::read_to_string(bindings_path)
        .expect("patch_unsized_arrays: failed to read bindings.rs");
    let mut patched = original;
    let mut changes = 0usize;
    for (sym, len) in entries {
        // const globals come out as `pub static NAME`, mutable ones
        // (PL_ppaddr, PL_check) as `pub static mut NAME`.
        let needles = [
            format!("pub static {sym}: ["),
            format!("pub static mut {sym}: ["),
        ];
        let Some((decl_start, needle)) = needles
            .iter()
            .find_map(|n| patched.find(n).map(|pos| (pos, n)))
        else {
            println!(
                "cargo:warning=patch_unsized_arrays: symbol {sym} not found in bindings.rs"
            );
            continue;
        };
        let scan_from = decl_start + needle.len();
        let Some(decl_end_rel) = patched[scan_from..].find("];") else {
            panic!("patch_unsized_arrays: malformed declaration for {sym}");
        };
        let decl_end = scan_from + decl_end_rel + 2;
        let snippet = &patched[decl_start..decl_end];
        if !snippet.contains("0usize") {
            // Already sized — nothing to do.
            continue;
        }
        let replaced = snippet.replacen("0usize", &format!("{len}usize"), 1);
        let mut new_src = String::with_capacity(patched.len() + 8);
        new_src.push_str(&patched[..decl_start]);
        new_src.push_str(&replaced);
        new_src.push_str(&patched[decl_end..]);
        patched = new_src;
        changes += 1;
    }
    if changes > 0 {
        std::fs::write(bindings_path, patched)
            .expect("patch_unsized_arrays: failed to write bindings.rs");
        println!(
            "# patched {changes} unsized PL_* arrays in bindings.rs (MAXO = {maxo})"
        );
    }
}

/// Read a `#define NAME N` integer macro from a C header file.
/// Returns `None` if not found or not parseable as an unsigned integer.
fn read_define_int(path: &Path, name: &str) -> Option<usize> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let Some(rest) = line.trim_start().strip_prefix("#define ") else {
            continue;
        };
        let mut parts = rest.trim_start().split_whitespace();
        if parts.next() != Some(name) {
            continue;
        }
        if let Some(value) = parts.next() {
            return value.parse().ok();
        }
    }
    None
}

/// GH-20 (b): threaded 風呼び出し規約 shim (`thx` モジュールの中身) を生成する。
///
/// 全関数を「先頭に my_perl を受ける」シグネチャで包み直す。元関数の
/// 先頭引数が `*mut PerlInterpreter` ならそのまま転送し、そうでなければ
/// 受け取った my_perl を捨てて残りを転送する。この規則は threading モード
/// を知らずに決まる (non-threaded では全関数が後者、threaded では context
/// 無し関数だけが後者) ので、`sys::thx::Perl_foo(my_perl, ...)` と書いた
/// 下流ソースは両モードで無改修のままコンパイルできる。
///
/// C variadic 関数 (Perl_croak 等) は stable Rust で包めないため省く。
fn generate_thx(funcs: &[FnDecl], thx_path: &Path) {
    let mut out = File::create(thx_path)
        .expect("Failed to create thx_bindings.rs");

    writeln!(out, "// This file is auto-generated by build.rs. Do not edit.").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "use crate::*;").unwrap();
    // macro_bindings.rs 冒頭のローカル prelude と同じもの。生成シグネチャは
    // bindgen / macrogen の綴りをそのまま写すため、素の c_char / ssize_t
    // 等がこのモジュール内でも解決できる必要がある
    writeln!(out, "#[allow(unused_imports)]").unwrap();
    writeln!(
        out,
        "use std::ffi::{{c_char, c_double, c_float, c_int, c_long, c_longlong, \
         c_schar, c_short, c_uchar, c_uint, c_ulong, c_ulonglong, c_ushort, c_void}};"
    )
    .unwrap();
    writeln!(out, "#[allow(non_camel_case_types, dead_code)] type ssize_t = isize;").unwrap();
    writeln!(out, "#[allow(non_camel_case_types, dead_code)] type size_t = usize;").unwrap();

    let mut seen = std::collections::HashSet::new();
    let mut n_emitted = 0usize;
    let mut n_variadic = 0usize;
    for (name, ret, args, is_variadic) in funcs {
        if !seen.insert(name.clone()) {
            continue;
        }
        if *is_variadic {
            n_variadic += 1;
            continue;
        }

        let has_ctx = args
            .first()
            .map(|(_, ty)| ty.replace(' ', "") == "*mutPerlInterpreter")
            .unwrap_or(false);

        // 無名引数 (Pat::Wild) に名前を振る
        let named: Vec<(String, String)> = args
            .iter()
            .enumerate()
            .map(|(i, (n, t))| {
                let n = if n.is_empty() { format!("__arg{i}") } else { n.clone() };
                (n, t.clone())
            })
            .collect();

        // context を取らない関数には捨てるだけの my_perl を前置する
        let ctx_param = if has_ctx {
            None
        } else {
            let mut p = "my_perl".to_string();
            while named.iter().any(|(n, _)| n == &p) {
                p.insert(0, '_');
            }
            Some(p)
        };

        let mut params: Vec<String> = Vec::new();
        if let Some(p) = &ctx_param {
            params.push(format!("{p}: *mut PerlInterpreter"));
        }
        params.extend(named.iter().map(|(n, t)| format!("{n}: {t}")));
        let ret_part = if ret == "()" { String::new() } else { format!(" -> {ret}") };

        writeln!(out).unwrap();
        writeln!(out, "/// thx shim for [`crate::{name}`].").unwrap();
        writeln!(out, "#[inline(always)]").unwrap();
        writeln!(out, "pub unsafe fn {name}({}){} {{", params.join(", "), ret_part).unwrap();
        if let Some(p) = &ctx_param {
            writeln!(out, "    let _ = {p};").unwrap();
        }
        let call_args: Vec<&str> = named.iter().map(|(n, _)| n.as_str()).collect();
        writeln!(out, "    unsafe {{ crate::{name}({}) }}", call_args.join(", ")).unwrap();
        writeln!(out, "}}").unwrap();
        n_emitted += 1;
    }

    println!(
        "# Generated thx_bindings.rs with {n_emitted} shims ({n_variadic} variadic functions skipped)"
    );
}

/// GH-20 (a): PERLVAR 観測 (macrogen が read 専用 `PL_xxx!` を生成するのと
/// 同じ情報源) から、読み書き両用の `PL_xxx_ptr!($my_perl)` を生成する。
/// `*mut T` を返すので `*PL_stack_sp_ptr!(p)` で読み、
/// `*PL_stack_sp_ptr!(p) = v` で書ける。展開形は read マクロと同じ規則:
///
/// | prefix | threaded build                    | non-threaded build       |
/// |--------|-----------------------------------|--------------------------|
/// | `I`    | `&raw mut (*$my_perl).I<name>`    | `&raw mut PL_<name>`     |
/// | `G`    | `&raw mut PL_<name>`              | `&raw mut PL_<name>`     |
///
/// PERLVARIC (const 宣言) はポインタ先が書き込み不可なので生成しない。
fn emit_perlvar_ptr_macros(
    path: &Path,
    dict: &libperl_macrogen::PerlvarDict,
    threaded: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use libperl_macrogen::PerlvarKind;

    let mut out = File::create(path)?;
    writeln!(out, "// This file is auto-generated by build.rs. Do not edit.")?;
    writeln!(out, "//")?;
    writeln!(out, "// PERLVAR pointer-accessor macros (GH-20). Same source of truth as")?;
    writeln!(out, "// the read-only PL_xxx! macros emitted by libperl-macrogen; formatted")?;
    writeln!(out, "// for this build's threading mode (no #[cfg] in the output).")?;

    let all_names: std::collections::HashSet<&str> =
        dict.iter().map(|e| e.name.as_str()).collect();

    for e in dict.iter() {
        if matches!(e.kind, PerlvarKind::Const { .. }) {
            continue;
        }
        // 例: PL_markstack の ptr 名は実在 perlvar markstack_ptr の read
        // マクロ PL_markstack_ptr! と衝突するため生成しない
        if all_names.contains(format!("{}_ptr", e.name).as_str()) {
            writeln!(out)?;
            writeln!(
                out,
                "// PL_{}_ptr! is NOT generated: the name would collide with the\n\
                 // read macro of the distinct perlvar `{}_ptr`.",
                e.name, e.name
            )?;
            continue;
        }
        let pl = format!("PL_{}", e.name);
        writeln!(out)?;
        writeln!(out, "/// `*mut` accessor for `{pl}` (read form: `{pl}!`).")?;
        writeln!(out, "#[macro_export]")?;
        writeln!(out, "macro_rules! {pl}_ptr {{")?;
        writeln!(out, "    ($my_perl:expr) => {{{{")?;
        if threaded && e.prefix == 'I' {
            writeln!(out, "        // type-check argument; evaluate exactly once")?;
            writeln!(out, "        let __my_perl: *mut $crate::PerlInterpreter = $my_perl;")?;
            writeln!(out, "        unsafe {{ &raw mut (*__my_perl).I{} }}", e.name)?;
        } else {
            writeln!(out, "        // discard $my_perl (global in this build); evaluate exactly once")?;
            writeln!(out, "        let _: *mut $crate::PerlInterpreter = $my_perl;")?;
            writeln!(out, "        unsafe {{ &raw mut $crate::{pl} }}")?;
        }
        writeln!(out, "    }}}};")?;
        writeln!(out, "}}")?;
    }
    Ok(())
}
