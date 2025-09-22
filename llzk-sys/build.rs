use anyhow::Result;
use llzk_sys_build_support::{
    build_llzk,
    config::{bindgen::BindgenConfig, cc::CCConfig},
    default::DefaultConfig,
    wrap_static_fns::WrapStaticFns,
};
use std::{env, path::Path};

//#[path = "src/build_support/mod.rs"]
//mod build_support;

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
    //let llzk = build_llzk(&DEFAULT_CFG)?;
    //let static_fns = WrapStaticFns::new()?;
    let cfg = (
        DEFAULT_CFG,
        build_llzk(&DEFAULT_CFG)?,
        WrapStaticFns::new()?,
    );
    cfg.generate()?
        //apply_bindgen_cfg(bindgen::builder(), &[&DEFAULT_CFG, &llzk, &static_fns])?
        //    .generate()?
        .write_to_file(Path::new(&env::var("OUT_DIR")?).join("bindings.rs"))?;

    //let mut cc = cc::Build::new();
    //apply_cc_cfg(&mut cc, &[&DEFAULT_CFG, &llzk, &static_fns])?;
    cfg.compile("llzk-sys-cc")
}

fn main() {
    if let Err(err) = run() {
        println!("cargo::error={err}");
    }
}
