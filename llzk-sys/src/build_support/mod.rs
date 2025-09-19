use anyhow::Result;
use bindgen::Builder;
use cc::Build;
use compile_commands::CompileCommands;
use config_traits::{BindgenConfig, CCConfig};
use default::DefaultConfig;
use llzk::LlzkBuild;

pub mod compile_commands;
pub mod config_traits;
pub mod default;
pub mod llzk;
pub mod mlir;
pub mod wrap_static_fns;

pub fn apply_bindgen_cfg(bindgen: Builder, cfgs: &[&dyn BindgenConfig]) -> Result<Builder> {
    cfgs.iter()
        .try_fold(bindgen, |bindgen, cfg| cfg.apply(bindgen))
}

pub fn apply_cc_cfg(cc: &mut Build, cfgs: &[&dyn CCConfig]) -> Result<()> {
    for cfg in cfgs {
        cfg.apply(cc)?;
    }
    Ok(())
}

fn build_llzk_inner(default_cfg: &DefaultConfig) -> Result<LlzkBuild> {
    if let Some(comp_db) = CompileCommands::get() {
        let llzk = LlzkBuild::build(&[default_cfg, &comp_db])?;
        comp_db.link(&llzk)?;
        return Ok(llzk);
    }
    LlzkBuild::build(&[default_cfg])
}

pub fn build_llzk(default_cfg: &DefaultConfig) -> Result<LlzkBuild> {
    let mut llzk = build_llzk_inner(default_cfg)?;
    for path in llzk.link_paths()? {
        println!("cargo:rustc-link-search={}", path.display());
    }
    for lib in llzk.library_names()? {
        println!("cargo:rustc-link-lib={lib}");
    }

    Ok(llzk)
}
