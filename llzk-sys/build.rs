use anyhow::Result;
use build_support::{
    apply_bindgen_cfg, apply_cc_cfg, build_llzk, default::DefaultConfig,
    wrap_static_fns::WrapStaticFns,
};
use std::{env, path::Path};

#[path = "src/build_support/mod.rs"]
mod build_support;

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

fn main() -> Result<()> {
    let llzk = build_llzk(&DEFAULT_CFG)?;
    let static_fns = WrapStaticFns::new()?;
    apply_bindgen_cfg(bindgen::builder(), &[&DEFAULT_CFG, &llzk, &static_fns])?
        .generate()?
        .write_to_file(Path::new(&env::var("OUT_DIR")?).join("bindings.rs"))?;

    let mut cc = cc::Build::new();
    apply_cc_cfg(&mut cc, &[&DEFAULT_CFG, &llzk, &static_fns])?;
    cc.compile("llzk-sys-cc");
    Ok(())
}
