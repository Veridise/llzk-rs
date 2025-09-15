use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use super::{func::FuncIO, Backend, Codegen};
#[cfg(feature = "lift-field-operations")]
use crate::ir::lift::{LiftIRGuard, LiftLike};
use crate::{
    gates::AnyQuery,
    halo2::{Expression, RegionIndex, Selector},
    io::AllCircuitIO,
    ir::{expr::Felt, stmt::IRStmt},
    synthesis::CircuitSynthesis,
    LoweringField,
};

use anyhow::Result;

use felt::FeltWrap;
use inner::PicusCodegenInner;
pub use lowering::PicusModuleLowering;
pub use params::{PicusParams, PicusParamsBuilder};
use picus::{opt::MutOptimizer as _, vars::VarStr};
use utils::mk_io;
use vars::{NamingConvention, VarKey, VarKeySeed};

mod felt;
mod inner;
mod lowering;
mod params;
mod utils;
mod vars;

pub type PicusBackend = Backend<PicusCodegen, InnerState>;
type InnerState = Rc<RefCell<PicusCodegenInner>>;
pub type PicusModule = picus::Module<VarKey>;
pub type PicusOutput = picus::Program<VarKey>;
type PipelineBuilder = picus::opt::OptimizerPipelineBuilder<VarKey>;
type Pipeline = picus::opt::OptimizerPipeline<VarKey>;

impl From<PicusParams> for InnerState {
    fn from(value: PicusParams) -> Self {
        Rc::new(RefCell::new(PicusCodegenInner::new(value)))
    }
}

#[derive(Clone)]
pub struct PicusCodegen {
    inner: InnerState,
}

impl PicusCodegen {
    fn naming_convention(&self) -> NamingConvention {
        self.inner.borrow().naming_convention()
    }

    fn var_consistency_check(&self, output: &PicusOutput) -> Result<()> {
        // Var consistency check
        for module in output.modules() {
            let vars = module.vars();
            // Get the set of io variables, without the fqn.
            // This set will have all the circuit cells that have been queried and resolved
            // during lowering.
            let io_vars = vars
                .keys()
                .filter_map(|k| match k {
                    VarKey::IO(func_io) => Some(*func_io),
                    _ => None,
                })
                .collect::<HashSet<_>>();

            // The set of io variables, with names, should be the same length.
            let io_var_count = vars
                .iter()
                .filter_map(|(k, v)| match k {
                    VarKey::IO(_) => Some(v),
                    _ => None,
                })
                .count();
            if io_vars.len() != io_var_count {
                // Inconsistency. Let's see which ones.
                let mut dups = HashMap::<FuncIO, Vec<&VarStr>>::new();
                for (k, v) in vars {
                    if let VarKey::IO(f) = k {
                        dups.entry(*f).or_default().push(v);
                    }
                }

                let dups = dups;
                for (k, names) in dups {
                    if names.len() == 1 {
                        continue;
                    }
                    log::error!("Mismatched variable! (key = {k:?}) (names = {names:?})");
                }
                anyhow::bail!(
                    "Inconsistency detected in circuit variables. Was expecting {} IO variables by {} were generated",
                    io_vars.len(),
                    io_var_count
                );
            }
        }
        Ok(())
    }

    fn optimization_pipeline(&self) -> Option<Pipeline> {
        self.inner.borrow().optimization_pipeline()
    }
}

impl<'c: 's, 's> Codegen<'c, 's> for PicusCodegen {
    type FuncOutput = PicusModuleLowering;
    type Output = PicusOutput;
    type State = InnerState;

    fn initialize(state: &'s Self::State) -> Self {
        Self {
            inner: state.clone(),
            //queue: Default::default(),
        }
    }

    //fn define_gate_function(
    //    &self,
    //    name: &str,
    //    selectors: &[&Selector],
    //    input_queries: &[AnyQuery],
    //    output_queries: &[AnyQuery],
    //) -> Result<Self::FuncOutput> {
    //    log::debug!("[Picus codegen::define_gate_function] selectors = {selectors:?}");
    //    log::debug!("[Picus codegen::define_gate_function] input_queries = {input_queries:?}");
    //    log::debug!("[Picus codegen::define_gate_function] output_queries = {output_queries:?}");
    //    let nc = self.naming_convention();
    //    self.inner.borrow_mut().add_module(
    //        name.to_owned(),
    //        mk_io(selectors.len() + input_queries.len(), VarKeySeed::arg, nc),
    //        mk_io(output_queries.len(), VarKeySeed::field, nc),
    //    )
    //}

    fn define_main_function(&self, io: AllCircuitIO) -> Result<Self::FuncOutput> {
        let ep = self.inner.borrow().entrypoint();
        let instance_io = io.instance;
        let advice_io = io.advice;
        let nc = self.naming_convention();
        self.inner.borrow_mut().add_module(
            ep,
            mk_io(
                instance_io.inputs().len() + advice_io.inputs().len(),
                VarKeySeed::arg,
                nc,
            ),
            mk_io(
                instance_io.outputs().len() + advice_io.outputs().len(),
                VarKeySeed::field,
                nc,
            ),
        )
    }

    fn on_scope_end(&self, _scope: Self::FuncOutput) -> Result<()> {
        log::debug!("Closing scope");
        Ok(())
    }

    fn generate_output(self) -> Result<Self::Output> {
        let mut output = PicusOutput::new(
            self.inner.borrow().prime().clone(),
            self.inner.borrow().modules().to_vec(),
        );
        self.var_consistency_check(&output)?;
        if let Some(mut opt) = self.optimization_pipeline() {
            opt.optimize(&mut output)?;
        }
        Ok(output)
    }

    fn define_function(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
    ) -> Result<Self::FuncOutput> {
        let nc = self.naming_convention();
        self.inner.borrow_mut().add_module(
            name.to_owned(),
            mk_io(inputs, VarKeySeed::arg, nc),
            mk_io(outputs, VarKeySeed::field, nc),
        )
    }
}
