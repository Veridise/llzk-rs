use anyhow::{Context, Result};
use llzk_sys_build_support::{
    build_llzk,
    config::{bindgen::BindgenConfig, cc::CCConfig},
    default::DefaultConfig,
    wrap_static_fns::WrapStaticFns,
};
use std::{env, path::Path};

const DEFAULT_CFG: DefaultConfig<'static> = DefaultConfig::new(
    &[
        "ArrayToScalar",
        "InlineIncludes",
        "Flattening",
        "RedundantOperationElimination",
        "RedundantReadAndWriteElimination",
        "UnusedDeclarationElimination",
        "FieldWriteValidator",
    ],
    &[
        "GetDialectHandle__llzk__.*",
        "DestroyOpBuilder.*",
        "CreateOpBuilder.*",
        "OpBuilder.*",
        "RegisterLLZK.*Passes",
    ],
    &[
        "OpBuilder",
        "OpBuilderListener",
        "Notify(Operation|Block)Inserted",
        "(Op|Block)InsertionPoint",
        "ValueRange",
    ],
);

fn run() -> Result<()> {
    let out_dir_var = env::var("OUT_DIR").with_context(|| "Loading OUT_DIR env variable failed")?;
    let out_dir = Path::new(&out_dir_var);
    let cfg = (
        DEFAULT_CFG,
        build_llzk(&DEFAULT_CFG).context("Failed to build LLZK")?,
        WrapStaticFns::new(out_dir),
    );
    cfg.generate()
        .context("Failed to generate Bindgen bindings")?
        .write_to_file(out_dir.join("bindings.rs"))
        .context("Failed to write bindings to file")?;

    cfg.compile("llzk-sys-cc")
        .context("Failed to build static functions")
}

fn main() {
    if let Err(err) = run() {
        println!("cargo::error={err:#}");
    }
}
