use anyhow::{Context as _, Result};
use bindgen::Builder;
use cc::Build;
use compile_commands::CompileCommands;
use config_traits::{BindgenConfig, CCConfig};
use default::DefaultConfig;
use llzk::LlzkBuild;

use crate::build_support::llzk::LIBDIR;

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
    let llzk = build_llzk_inner(default_cfg)?;
    // TODO: Change the path `llzk` keeps track of to be $OUT instead of the current: $OUT/build
    // Since the libs are actually installed in $OUT/$LIBDIR.
    let lib_path = llzk
        .path()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to extract parent of {}", llzk.path().display()))?
        .join(LIBDIR);
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    // Adding the whole archive modifier is optional since only seems to be required for some GNU-like linkers.
    let modifier = if std::env::var("LLZK_SYS_ENABLE_WHOLE_ARCHIVE").is_ok_and(|var| var == "1") {
        ":+whole-archive"
    } else {
        ""
    };
    for entry in lib_path
        .read_dir()
        .with_context(|| format!("Failed to read directory {}", lib_path.display()))?
    {
        let name = entry
            .context("Failed to read entry in directory")?
            .file_name()
            .into_string()
            .map_err(|orig| anyhow::anyhow!("Failed to convert {orig:?} into a String"))?;
        if let Some(lib) = name.strip_prefix("lib").and_then(|s| s.strip_suffix(".a")) {
            println!("cargo:rustc-link-lib=static{modifier}={lib}");
        }
    }

    Ok(llzk)
}
