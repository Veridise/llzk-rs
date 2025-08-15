use std::{borrow::Cow, collections::HashMap, marker::PhantomData};

#[cfg(feature = "lift-field-operations")]
use crate::ir::lift::{LiftIRGuard, LiftLike};
use crate::{
    backend::picus::PicusModule,
    expressions::ScopedExpression,
    halo2::{Expression, Field, RegionIndex, Selector},
    ir::stmt::IRStmt,
    synthesis::{
        regions::{RegionIndexToStart, FQN},
        CircuitSynthesis,
    },
};
use crate::{
    backend::{
        func::FuncIO,
        lowering::{
            lowerable::{Lowerable, LoweringOutput},
            Lowering,
        },
        picus::{felt::FeltWrap, params::PicusParams, Pipeline, PipelineBuilder},
        resolvers::{
            QueryResolver, ResolvedQuery, ResolvedSelector, ResolversProvider, SelectorResolver,
        },
    },
    LoweringField,
};

use anyhow::{anyhow, Result};

pub use super::lowering::PicusModuleLowering;
use super::lowering::PicusModuleRef;
use super::vars::{NamingConvention, VarKey, VarKeySeed, VarKeySeedInner};
use midnight_halo2_proofs::plonk::{AdviceQuery, FixedQuery, InstanceQuery};
use picus::{
    opt::passes::{ConsolidateVarNamesPass, EnsureMaxExprSizePass, FoldExprsPass},
    vars::VarStr,
    ModuleWithVars as _,
};

pub struct PicusCodegenInner<L> {
    params: PicusParams,
    modules: Vec<PicusModuleRef>,
    current_scope: Option<PicusModuleLowering<L>>,
    enqueued_stmts: HashMap<RegionIndex, Vec<IRStmt<Expression<L>>>>,
    _marker: PhantomData<L>,
}

impl<F> PicusCodegenInner<F> {
    pub fn new(params: PicusParams) -> Self {
        Self {
            params,
            modules: Default::default(),
            current_scope: Default::default(),
            enqueued_stmts: Default::default(),
            _marker: Default::default(),
        }
    }
}

impl<F: LoweringField> PicusCodegenInner<F> {
    pub fn naming_convention(&self) -> NamingConvention {
        self.params.naming_convention()
    }

    pub fn modules(&self) -> &[PicusModuleRef] {
        &self.modules
    }

    pub fn optimization_pipeline(&self) -> Option<Pipeline<F>> {
        if !self.params.optimize() {
            return None;
        }
        let mut pipeline = PipelineBuilder::<F>::new()
            .add_pass::<FoldExprsPass<FeltWrap<F>>>()
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
        syn: Option<&CircuitSynthesis<F>>,
    ) -> Result<PicusModuleLowering<F>>
    where
        O: Into<VarKey> + Into<VarStr> + Clone,
    {
        let regions = syn.map(|syn| syn.regions_by_index());
        log::debug!("Region data: {regions:?}");
        let module = PicusModule::shared(name.clone(), inputs, outputs);
        if let Some(syn) = syn {
            module
                .borrow_mut()
                .add_vars(syn.seen_advice_cells().map(|((col, row), name)| {
                    VarKeySeed::new(
                        VarKeySeedInner::IO(FuncIO::Advice(*col, *row), Some(Cow::Borrowed(name))),
                        self.params.naming_convention(),
                    )
                }));
        }
        self.modules.push(module.clone());
        let scope = PicusModuleLowering::new(
            module,
            #[cfg(feature = "lift-field-operations")]
            self.params.lift_fixed(),
            regions,
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
