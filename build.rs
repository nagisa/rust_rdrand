use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    compile_llvm("src/impls.ll", "librdrand_impls.a");
    println!("cargo:rustc-cfg=has_impls");
    println!("cargo:rustc-link-lib=static=rdrand_impls");
    println!("cargo:rustc-link-search=native={}", ::std::env::var("OUT_DIR").unwrap());
}

fn compile_llvm<P: AsRef<Path>>(path: P, to: P) {
    // let to = to.as_ref();
    let outpath: PathBuf = ::std::env::var_os("OUT_DIR").expect("no $OUT_DIR").into();
    let objfile = outpath.join(path.as_ref().with_extension("o")
                               .file_name().expect("input has no filename"));
    let afile = outpath.join(to);
    assert!(Command::new("llc").arg(path.as_ref())
                               .arg("-O3")
                               .arg("-march=x86-64")
                               .arg("-mattr=+rdrnd,+rdseed")
                               .arg("-filetype=obj")
                               .arg("-o")
                               .arg(&objfile)
                               .status().expect("could not spawn opt").success(),
                               "llc did not exit successfully");

    assert!(Command::new("llvm-ar").arg("rcs")
                                   .arg(afile)
                                   .arg(objfile)
                                   .status().expect("could not spawn llvm-ar").success(),
                                   "llvm-ar did not exit successfully");
}
