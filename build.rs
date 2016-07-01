extern crate llvm_build_utils;

use std::path::{PathBuf, Path};
use llvm_build_utils::*;

fn main() {
    let known_triple_starts = ["x86_64"];
    if let Ok(var) = ::std::env::var("TARGET") {
        if !known_triple_starts.iter().any(|x| var.starts_with(x)){
            return;
        }
    }
    let outpath: PathBuf = ::std::env::var_os("OUT_DIR").expect("no $OUT_DIR").into();
    build_archive(&outpath.join("librdrand_impls.a") as &AsRef<Path>, &[
    (&"src/impls.ll" as &AsRef<Path>, BuildOptions {
        attr: "+rdrnd,+rdseed".into(),
        ..BuildOptions::default()
    })]).expect("error happened");
    println!("cargo:rustc-cfg=has_impls");
    println!("cargo:rustc-link-lib=static=rdrand_impls");
    println!("cargo:rustc-link-search=native={}", ::std::env::var("OUT_DIR").unwrap());
}
