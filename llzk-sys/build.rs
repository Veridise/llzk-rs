use cmake::Config;
use glob::glob;
use std::{
    env,
    error::Error,
    ffi::OsStr,
    fs::read_dir,
    path::Path,
    process::{exit, Command},
    str,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error);
        exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let llzk_path = Config::new("llzk-lib")
        .define("LLZK_BUILD_DEVTOOLS", "OFF")
        .define("BUILD_TESTING", "OFF")
        .define("LLVM_DIR", env::var("DEP_MLIR_PREFIX")?)
        .define("MLIR_DIR", env::var("DEP_MLIR_PREFIX")?)
        //.build_target("LLZKDialect")
        //.build_target("LLZKDialectRegistration")
        .build();
    eprintln!("llzk_path = {}", llzk_path.display());
    println!("cargo:rustc-link-search={}/build/lib", llzk_path.display());

    for f in glob(llzk_path.join("**/*.a").to_str().unwrap())? {
        let f = f?;
        eprintln!("=== {}", f.display());
    }
    println!("cargo:rustc-link-lib=LLZKDialect");
    println!("cargo:rustc-link-lib=LLZKDialectRegistration");

    bindgen::builder()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", llzk_path.join("build/include").display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()?
        .write_to_file(Path::new(&env::var("OUT_DIR")?).join("bindings.rs"))?;

    Ok(())
}
