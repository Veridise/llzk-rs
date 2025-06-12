use std::fmt;

use crate::halo2::{
    Advice, AdviceQuery, Any, Column, ColumnType, Field, FixedQuery, Gate, Instance, InstanceQuery,
    PrimeField, Rotation, Selector,
};
use anyhow::{bail, Result};
use backend::picus::PicusBackend;
use backend::{lowering::Lowering, Backend};
use gates::compute_gate_arity;
use ir::CircuitStmt;
use synthesis::regions::{RegionRow, Row};
use synthesis::CircuitSynthesis;

pub(crate) mod backend;
mod gates;
mod halo2;
mod io;
mod ir;
mod synthesis;
#[cfg(test)]
mod test;
mod value;

pub use backend::picus::PicusOutput;
pub use backend::picus::PicusParams;
pub use io::{CircuitIO, CircuitWithIO};

pub fn picus_codegen_with_params<F, C>(circuit: &C, params: PicusParams) -> Result<PicusOutput<F>>
where
    F: Field,
    C: CircuitWithIO<F>,
{
    let backend = PicusBackend::initialize(params);
    backend.codegen_impl(circuit)
}

pub fn picus_codegen<F, C>(circuit: &C) -> Result<PicusOutput<F>>
where
    F: Field,
    C: CircuitWithIO<F>,
{
    picus_codegen_with_params(circuit, Default::default())
}
