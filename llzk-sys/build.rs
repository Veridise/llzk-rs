use cmake::Config;
use glob::glob;
use std::{
    collections::HashSet,
    env,
    error::Error,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::exit,
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
    let mlir_cmake_path = env::var("DEP_MLIR_CMAKE_DIR")?;
    let llzk_src_path = Path::new("llzk-lib");
    let llzk_path = Config::new(&llzk_src_path)
        .define("LLZK_BUILD_DEVTOOLS", "OFF")
        .define("BUILD_TESTING", "OFF")
        .define("LLVM_DIR", &mlir_cmake_path)
        .define("MLIR_DIR", &mlir_cmake_path)
        .define("LLVM_ROOT", &mlir_path)
        .define("MLIR_ROOT", &mlir_path)
        .build()
        .join("build");

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

    let include_paths = [llzk_src_path, &llzk_path, Path::new(&mlir_path)];
    let passes = [
        "ArrayToScalar",
        "InlineIncludes",
        "Flattening",
        "RedundantOperationElimination",
        "RedundantReadAndWriteElimination",
        "UnusedDeclarationElimination",
        "FieldWriteValidator",
    ];

    let builder = bindgen::builder()
        .header("wrapper.h")
        .clang_args(include_paths.map(|path| format!("-I{}", path.join("include").display())))
        .allowlist_item("[Ll]lzk.*")
        .allowlist_function("mlirGetDialectHandle__llzk__.*")
        .allowlist_recursively(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    let builder = passes.iter().fold(builder, |builder, pass| {
        builder.allowlist_function(format!("mlir(Create|Register).*{pass}Pass"))
    });

    builder
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
