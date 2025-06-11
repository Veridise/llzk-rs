use std::marker::PhantomData;

use super::Backend;
use crate::{
    gates::AnyQuery,
    halo2::{Advice, Instance, PrimeField, Selector},
    CircuitIO,
};
use anyhow::Result;

mod expr;
mod lowering;
mod vars;

use lowering::PicusModule;
pub use lowering::PicusModuleLowering;

pub struct PicusParams {
    expr_cutoff: usize,
}

impl Default for PicusParams {
    fn default() -> Self {
        Self { expr_cutoff: 10 }
    }
}

pub struct PicusBackend<F> {
    params: PicusParams,
    modules: Vec<PicusModule>,
    _marker: PhantomData<F>,
}

pub struct PicusOutput {}

impl<'c, F: PrimeField> Backend<'c, PicusParams, PicusOutput> for PicusBackend<F> {
    type FuncOutput = PicusModuleLowering<F>;
    type F = F;

    fn initialize(params: PicusParams) -> Self {
        Self {
            params,
            modules: Default::default(),
            _marker: Default::default(),
        }
    }

    fn generate_output(&'c self) -> Result<PicusOutput> {
        todo!()
    }

    fn define_gate_function<'f>(
        &'c self,
        _name: &str,
        _selectors: &[&Selector],
        _queries: &[AnyQuery],
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        todo!()
    }

    fn define_main_function<'f>(
        &'c self,
        _advice_io: &CircuitIO<Advice>,
        _instance_io: &CircuitIO<Instance>,
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        todo!()
    }
}
