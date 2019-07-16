use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use failure::{Error, ResultExt};
use lazy_static::lazy_static;

const QUICKJS_DIR: &str = "quickjs";
const QUICKJS_SRC: &str = "quickjs-2019-07-09.tar.xz";

lazy_static! {
    static ref OUT_DIR: PathBuf = env::var_os("OUT_DIR").expect("OUT_DIR").into();
    static ref CARGO_MANIFEST_DIR: PathBuf = env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR")
        .into();
}

fn build_libquickjs() -> Result<(), Error> {
    let quickjs_dir = OUT_DIR.join(QUICKJS_DIR);

    if !quickjs_dir.join("quickjs.h").is_file() {
        let quickjs_src = CARGO_MANIFEST_DIR.join(QUICKJS_SRC).canonicalize()?;

        println!(
            "extract quickjs from {:?} to {:?}",
            quickjs_src, quickjs_dir
        );

        fs::create_dir_all(&quickjs_dir)?;

        Command::new("tar")
            .arg("xvf")
            .arg(&quickjs_src)
            .arg("-C")
            .arg(&quickjs_dir)
            .args(&["--strip-components", "1"])
            .output()?;
    }

    if cfg!(target_os = "macos") {
        let quickjs_patch = CARGO_MANIFEST_DIR
            .join("quickjs.macos.patch")
            .canonicalize()?;

        println!("patch Makefile for macos with {:?}", quickjs_patch);

        Command::new("patch")
            .current_dir(&quickjs_dir)
            .arg("Makefile")
            .arg(quickjs_patch)
            .output()?;
    }

    if !quickjs_dir.join("libquickjs.a").is_file() {
        println!("build quickjs ...");

        Command::new("make").current_dir(&quickjs_dir).output()?;
    }

    println!(
        "cargo:rustc-link-search=native={}",
        quickjs_dir.to_string_lossy()
    );
    println!("cargo:rustc-link-lib=static=quickjs");

    Ok(())
}

#[cfg(feature = "gen")]
fn gen_binding_files() -> Result<(), Error> {
    use failure::err_msg;

    let quickjs_dir = OUT_DIR.join(QUICKJS_DIR);
    let raw_file = OUT_DIR.join("raw.rs");

    println!("generating binding files to {:?}", raw_file);

    bindgen::builder()
        .header(quickjs_dir.join("quickjs-libc.h").to_string_lossy())
        .clang_arg(format!("-I{}", quickjs_dir.to_string_lossy()))
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
