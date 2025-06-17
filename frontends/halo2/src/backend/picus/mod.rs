use std::{cell::RefCell, marker::PhantomData};

use super::Backend;
use crate::{
    gates::AnyQuery,
    halo2::{Advice, Field, Instance, Selector},
    CircuitIO,
};
use anyhow::Result;

mod expr;
mod lowering;
mod output;
mod stmt;
mod vars;

pub use lowering::PicusModuleLowering;
use lowering::PicusModuleRef;
use output::PicusModule;
pub use output::PicusOutput;

pub struct PicusParams {
    expr_cutoff: usize,
    entrypoint: String,
}

impl PicusParams {
    pub fn builder() -> PicusParamsBuilder {
        PicusParamsBuilder(Default::default())
    }
}

pub struct PicusParamsBuilder(PicusParams);

impl PicusParamsBuilder {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn expr_cutoff(self, expr_cutoff: usize) -> Self {
        let mut p = self.0;
        p.expr_cutoff = expr_cutoff;
        Self(p)
    }

    pub fn extrypoint(self, name: &str) -> Self {
        let mut p = self.0;
        p.entrypoint = name.to_owned();
        Self(p)
    }
}

impl Into<PicusParams> for PicusParamsBuilder {
    fn into(self) -> PicusParams {
        self.0
    }
}

impl Default for PicusParams {
    fn default() -> Self {
        Self {
            expr_cutoff: 10,
            entrypoint: "Main".to_owned(),
        }
    }
}

pub struct PicusBackend<F> {
    params: PicusParams,
    modules: RefCell<Vec<PicusModuleRef>>,
    _marker: PhantomData<F>,
}

impl<'c, F: Field> Backend<'c, PicusParams, PicusOutput<F>> for PicusBackend<F> {
    type FuncOutput = PicusModuleLowering<F>;
    type F = F;

    fn initialize(params: PicusParams) -> Self {
        Self {
            params,
            modules: Default::default(),
            _marker: Default::default(),
        }
    }

    fn generate_output(&'c self) -> Result<PicusOutput<Self::F>> {
        let output = PicusOutput::from(self.modules.borrow().clone());

        // TODO: Cut the expressions that are too big
        Ok(output)
    }

    fn define_gate_function<'f>(
        &'c self,
        name: &str,
        selectors: &[&Selector],
        queries: &[AnyQuery],
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        let module = PicusModule::shared(name.to_owned(), selectors.len() + queries.len(), 0);
        self.modules.borrow_mut().push(module.clone());
        Ok(Self::FuncOutput::from(module))
    }

    fn define_main_function<'f>(
        &'c self,
        advice_io: &CircuitIO<Advice>,
        instance_io: &CircuitIO<Instance>,
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        let module = PicusModule::shared(
            self.params.entrypoint.clone(),
            instance_io.inputs().len() + advice_io.inputs().len(),
            instance_io.outputs().len() + advice_io.outputs().len(),
        );
        self.modules.borrow_mut().push(module.clone());
        Ok(Self::FuncOutput::from(module))
    }
}
