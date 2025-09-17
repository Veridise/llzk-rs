use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use super::{func::FuncIO, Backend, Codegen};
use crate::{
    io::{AdviceIO, InstanceIO},
    ir::expr::Felt,
};

use anyhow::Result;

use inner::PicusCodegenInner;
pub use lowering::PicusModuleLowering;
pub use params::PicusParams;
use picus::{opt::MutOptimizer as _, vars::VarStr};
use utils::mk_io;
use vars::{NamingConvention, VarKey, VarKeySeed};

mod felt;
mod inner;
mod lowering;
pub(crate) mod params;
mod utils;
mod vars;

pub type PicusBackend = Backend<PicusCodegen, InnerState>;
type InnerState = Rc<RefCell<PicusCodegenInner>>;
pub type PicusModule = picus::Module<VarKey>;
/// Output produced by the picus backend.
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
        }
    }

    fn set_prime_field(&self, prime: Felt) -> Result<()> {
        self.inner.borrow_mut().set_prime(prime);
        Ok(())
    }

    fn define_main_function(
        &self,
        advice_io: &AdviceIO,
        instance_io: &InstanceIO,
    ) -> Result<Self::FuncOutput> {
        let ep = self.inner.borrow().entrypoint();
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
            self.inner.borrow().prime()?,
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
