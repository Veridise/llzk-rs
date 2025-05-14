use cmake::Config;
use glob::glob;
use std::{
    collections::HashSet,
    env,
    error::Error,
    ffi::OsStr,
    fs::read_dir,
    path::{Path, PathBuf},
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
    let mlir_path = env::var("DEP_MLIR_PREFIX")?;
    let llzk_src_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("llzk-lib");
    let llzk_path = Config::new("llzk-lib")
        .define("LLZK_BUILD_DEVTOOLS", "OFF")
        .define("BUILD_TESTING", "OFF")
        .define("LLVM_DIR", &mlir_path)
        .define("MLIR_DIR", &mlir_path)
        .build_target("LLZKDialectRegistration")
        .build_target("LLZKTransforms")
        .build_target("LLZKValidators")
        .build();
    eprintln!("llzk_path = {}", llzk_path.display());
    eprintln!("llzk_src_path = {}", llzk_src_path.display());

    let mut seen: HashSet<PathBuf> = Default::default();
    for f in glob(llzk_path.join("**/*.a").to_str().unwrap())? {
        let f = f?;
        if let Some(parent) = f.parent().and_then(|p| Some(p.to_path_buf())) {
            if !seen.contains(&parent) {
                println!("cargo:rustc-link-search={}", parent.display());
                seen.insert(parent);
            }
        }
        if let Some(name) = f
            .file_name()
            .and_then(OsStr::to_str)
            .and_then(parse_archive_name)
        {
            println!("cargo:rustc-link-lib={}", name);
        }
    }

    bindgen::builder()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", llzk_src_path.join("include").display()))
        .clang_arg(format!("-I{}", llzk_path.join("build/include").display()))
        .clang_arg(format!(
            "-I{}",
            Path::new(&mlir_path).join("include").display()
        ))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()?
        .write_to_file(Path::new(&env::var("OUT_DIR")?).join("bindings.rs"))?;

    Ok(())
}

fn parse_archive_name(name: &str) -> Option<&str> {
    if let Some(name) = name.strip_prefix("lib") {
        name.strip_suffix(".a")
    } else {
        None
    }
}
