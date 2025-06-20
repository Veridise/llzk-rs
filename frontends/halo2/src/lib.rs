use crate::halo2::PrimeField;
use anyhow::Result;
use backend::picus::PicusBackend;
use backend::Backend;
use backend::InlineConstraintsStrat;

mod arena;
pub(crate) mod backend;
mod gates;
mod halo2;
mod io;
mod ir;
mod synthesis;
#[cfg(test)]
mod test;
mod value;

pub use crate::ir::lift::Lift;
pub use backend::picus::PicusOutput;
pub use backend::picus::PicusParams;
pub use backend::picus::PicusParamsBuilder;
pub use io::{CircuitIO, CircuitWithIO};

pub fn picus_codegen_with_params<F, C>(
    circuit: &C,
    params: PicusParams,
) -> Result<PicusOutput<Lift<F>>>
where
    F: PrimeField,
    C: CircuitWithIO<Lift<F>>,
{
    let backend = PicusBackend::initialize(params);
    backend.codegen(circuit, &InlineConstraintsStrat)
}

pub fn picus_codegen<F, C>(circuit: &C) -> Result<PicusOutput<Lift<F>>>
where
    F: PrimeField,
    C: CircuitWithIO<Lift<F>>,
{
    picus_codegen_with_params(circuit, Default::default())
}
