use anyhow::Result;
use compile_commands::CompileCommands;
use default::DefaultConfig;
use llzk::LlzkBuild;
use std::path::Path;

pub mod compile_commands;
pub mod config;
pub mod default;
pub mod llzk;
pub mod mlir;
pub mod wrap_static_fns;

pub fn build_llzk(default_cfg: &DefaultConfig) -> Result<LlzkBuild<'static>> {
    let comp_db = CompileCommands::get();
    let llzk = LlzkBuild::build((default_cfg, &comp_db), Path::new("llzk-lib"))?;
    if let Some(comp_db) = comp_db {
        comp_db.link(&llzk)?;
    }
    llzk.emit_cargo_instructions()?;
    Ok(llzk)
}
