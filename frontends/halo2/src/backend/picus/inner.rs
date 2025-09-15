use crate::backend::picus::PicusModule;
use crate::backend::picus::{params::PicusParams, Pipeline, PipelineBuilder};
#[cfg(feature = "lift-field-operations")]
use crate::ir::lift::{LiftIRGuard, LiftLike};

use anyhow::Result;

pub use super::lowering::PicusModuleLowering;
use super::lowering::PicusModuleRef;
use super::vars::{NamingConvention, VarKey};
use picus::{
    opt::passes::{ConsolidateVarNamesPass, EnsureMaxExprSizePass, FoldExprsPass},
    vars::VarStr,
};

pub struct PicusCodegenInner {
    params: PicusParams,
    modules: Vec<PicusModuleRef>,
    current_scope: Option<PicusModuleLowering>,
}

impl PicusCodegenInner {
    pub fn new(params: PicusParams) -> Self {
        Self {
            params,
            modules: Default::default(),
            current_scope: Default::default(),
        }
    }
}

impl PicusCodegenInner {
    pub fn naming_convention(&self) -> NamingConvention {
        self.params.naming_convention()
    }

    pub fn modules(&self) -> &[PicusModuleRef] {
        &self.modules
    }

    pub fn prime(&self) -> &picus::felt::Felt {
        self.params.prime()
    }

    pub fn optimization_pipeline(&self) -> Option<Pipeline> {
        if !self.params.optimize() {
            return None;
        }
        let mut pipeline = PipelineBuilder::new()
            .add_pass::<FoldExprsPass>()
            .add_pass::<ConsolidateVarNamesPass>();
        if let Some(expr_cutoff) = self.params.expr_cutoff() {
            pipeline = pipeline.add_pass_with_params::<EnsureMaxExprSizePass<NamingConvention>>((
                expr_cutoff,
                self.naming_convention(),
            ))
        }
        Some(pipeline.into())
    }

    pub fn add_module<O>(
        &mut self,
        name: String,
        inputs: impl Iterator<Item = O>,
        outputs: impl Iterator<Item = O>,
    ) -> Result<PicusModuleLowering>
    where
        O: Into<VarKey> + Into<VarStr> + Clone,
    {
        let module = PicusModule::shared(name.clone(), inputs, outputs);

        self.modules.push(module.clone());
        let scope = PicusModuleLowering::new(
            module,
            #[cfg(feature = "lift-field-operations")]
            self.params.lift_fixed(),
            self.params.naming_convention(),
        );
        log::debug!("Setting the scope to {name}");
        self.current_scope = Some(scope.clone());
        Ok(scope)
    }

    pub fn entrypoint(&self) -> String {
        self.params.entrypoint().to_owned()
    }
}
