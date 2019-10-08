use std::env;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use failure::{Error, ResultExt};
use lazy_static::lazy_static;
use regex::Regex;

const QUICKJS_SRC: &str = "quickjs-2019-09-18.tar.xz";

lazy_static! {
    static ref OUT_DIR: PathBuf = env::var_os("OUT_DIR").expect("OUT_DIR").into();
    static ref CARGO_MANIFEST_DIR: PathBuf = env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR")
        .into();
    static ref QUICKJS_DIR: PathBuf = OUT_DIR.join(QUICKJS_SRC.split('.').next().unwrap());
}

fn unpack_source_files(quickjs_src: &Path, out_dir: &Path) -> Result<(), Error> {
    println!("extract `quickjs` from {:?} to {:?}", quickjs_src, out_dir);

    let f = fs::File::open(quickjs_src)?;
    let r = lzma::LzmaReader::new_decompressor(f)?;
    tar::Archive::new(r).unpack(&out_dir)?;

    Ok(())
}

fn patch_makefile(makefile: &Path) -> Result<(), Error> {
    let content = fs::read_to_string(makefile)?;

    let content = if cfg!(feature = "debug") {
        Regex::new("^CFLAGS_OPT=(.*) -O2\n")?.replace(&content, "CFLAGS_OPT=$1 -O0 -g\n")
    } else {
        content.into()
    };

    let content = if cfg!(feature = "pic") {
        content
            .replace("CFLAGS+=$(DEFINES)\n", "CFLAGS+=$(DEFINES) -fPIC\n")
            .into()
    } else {
        content
    };

    fs::rename(makefile, makefile.with_extension("bak"))?;
    fs::write(makefile, content.as_bytes())?;

    Ok(())
}

fn patch_quickjs(quickjs: &Path) -> Result<(), Error> {
    let mut content = fs::read_to_string(quickjs)?;

    if cfg!(feature = "dump_free") {
        content = content.replace("//#define DUMP_FREE\n", "#define DUMP_FREE\n");
    }
    if cfg!(feature = "dump_closure") {
        content = content.replace("//#define DUMP_CLOSURE\n", "#define DUMP_CLOSURE\n");
    }
    if cfg!(feature = "dump_bytecode") {
        content = content.replace("//#define DUMP_BYTECODE", "#define DUMP_BYTECODE");
    }
    if cfg!(feature = "dump_gc") {
        content = content.replace("//#define DUMP_GC\n", "#define DUMP_GC\n");
    }
    if cfg!(feature = "dump_gc_free") {
        content = content.replace("//#define DUMP_GC_FREE\n", "#define DUMP_GC_FREE\n");
    }
    if cfg!(feature = "dump_leaks") {
        content = content.replace("//#define DUMP_LEAKS", "#define DUMP_LEAKS");
    }
    if cfg!(feature = "dump_mem") {
        content = content.replace("//#define DUMP_MEM\n", "#define DUMP_MEM\n");
    }
    if cfg!(feature = "dump_objects") {
        content = content.replace("//#define DUMP_OBJECTS", "#define DUMP_OBJECTS");
    }
    if cfg!(feature = "dump_atoms") {
        content = content.replace("//#define DUMP_ATOMS", "#define DUMP_ATOMS");
    }
    if cfg!(feature = "dump_shapes") {
        content = content.replace("//#define DUMP_SHAPES", "#define DUMP_SHAPES");
    }
    if cfg!(feature = "dump_module_resolve") {
        content = content.replace(
            "//#define DUMP_MODULE_RESOLVE\n",
            "#define DUMP_MODULE_RESOLVE\n",
        );
    }
    if cfg!(feature = "dump_promise") {
        content = content.replace("//#define DUMP_PROMISE\n", "#define DUMP_PROMISE\n");
    }
    if cfg!(feature = "dump_read_object") {
        content = content.replace("//#define DUMP_READ_OBJECT\n", "#define DUMP_READ_OBJECT\n");
    }

    fs::rename(quickjs, quickjs.with_extension("bak"))?;
    fs::write(quickjs, content.as_bytes())?;

    Ok(())
}

fn patch_quickjs_libc(quickjs_libc: &Path) -> Result<(), Error> {
    let mut content = fs::read_to_string(quickjs_libc)?;

    if cfg!(target_os = "macos") {
        content = content
            .replace("(&st.st_atim)", "(&st.st_atimespec)")
            .replace("(&st.st_mtim)", "(&st.st_mtimespec)")
            .replace("(&st.st_ctim)", "(&st.st_ctimespec)");
    }

    fs::rename(quickjs_libc, quickjs_libc.with_extension("bak"))?;
    fs::write(quickjs_libc, content.as_bytes())?;

    Ok(())
}

fn build_libquickjs() -> Result<(), Error> {
    if !QUICKJS_DIR.join("quickjs.h").is_file() {
        unpack_source_files(
            &CARGO_MANIFEST_DIR.join(QUICKJS_SRC).canonicalize()?,
            OUT_DIR.as_path(),
        )?;
    }

    if !OUT_DIR.join("VERSION").is_file() {
        fs::copy(QUICKJS_DIR.join("VERSION"), OUT_DIR.join("VERSION"))?;
    }

    patch_makefile(&QUICKJS_DIR.join("Makefile"))?;
    patch_quickjs(&QUICKJS_DIR.join("quickjs.c"))?;
    patch_quickjs_libc(&QUICKJS_DIR.join("quickjs-libc.c"))?;

    let repl_c = if cfg!(feature = "bignum") {
        "repl-bn.c"
    } else {
        "repl.c"
    };
    let qjscalc_c = "qjscalc.c";

    let quickjs = format!(
        "quickjs{}{}",
        if cfg!(feature = "bignum") { ".bn" } else { "" },
        if cfg!(feature = "lto") { ".lto" } else { "" }
    );
    let libquickjs = format!("lib{}.a", quickjs);
    let mut targets = vec![libquickjs];

    if cfg!(feature = "repl") {
        targets.push(repl_c.to_owned());
    }

    if cfg!(feature = "qjscalc") {
        targets.push(qjscalc_c.to_owned());
    }

    for target in &targets {
        if !QUICKJS_DIR.join(target).is_file() {
            println!("make {:?} ...", target);

            let output = Command::new("make")
                .arg(target)
                .current_dir(QUICKJS_DIR.as_path())
                .output()?;

            println!("status: {}", output.status);
            println!("stdout: {}", CString::new(output.stdout)?.to_string_lossy());
            eprintln!("stderr: {}", CString::new(output.stderr)?.to_string_lossy());
        }
    }

    println!(
        "cargo:rustc-link-search=native={}",
        QUICKJS_DIR.to_string_lossy()
    );
    println!("cargo:rustc-link-lib=static={}", quickjs);
    println!("cargo:rerun-if-changed={}", QUICKJS_SRC);

    if cfg!(feature = "repl") {
        cc::Build::new()
            .file(QUICKJS_DIR.join(repl_c))
            .compile("repl");
    }

    if cfg!(feature = "qjscalc") {
        cc::Build::new()
            .file(QUICKJS_DIR.join(qjscalc_c))
            .compile("qjscalc");
    }

    Ok(())
}

#[cfg(feature = "gen")]
fn gen_binding_files() -> Result<(), Error> {
    use failure::err_msg;

    let raw_file = OUT_DIR.join("raw.rs");

    println!("generating binding files to {:?}", raw_file);

    bindgen::builder()
        .header(QUICKJS_DIR.join("quickjs-libc.h").to_string_lossy())
        .clang_arg(format!("-I{}", QUICKJS_DIR.to_string_lossy()))
        .whitelist_var("JS_.*")
        .whitelist_type("JS.*")
        .whitelist_function("(__)?(JS|JS|js)_.*")
        .opaque_type("FILE")
        .blacklist_type("__.*")
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .generate()
        .map_err(|_| err_msg("generate binding file"))?
        .write_to_file(raw_file)
        .context("write binding file")?;

    Ok(())
}

#[cfg(not(feature = "gen"))]
fn gen_binding_files() -> Result<(), Error> {
    Ok(())
}

fn main() -> Result<(), Error> {
    match &env::var("CARGO") {
        Ok(path) if path.ends_with("rls") => {}
        _ => {
            build_libquickjs().context("build quickjs library")?;
            gen_binding_files().context("generate binding files")?;
        }
    };

    Ok(())
}
