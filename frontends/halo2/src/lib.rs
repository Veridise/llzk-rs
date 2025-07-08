use crate::halo2::PrimeField;
use anyhow::Result;
use backend::picus::PicusBackend;
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

pub use crate::ir::lift::{Lift, LiftLike};
pub use backend::events::{EmitStmtsMessage, EventReceiver, EventSender};
pub use backend::picus::PicusOutput;
pub use backend::picus::PicusParams;
pub use backend::picus::PicusParamsBuilder;
pub use backend::Backend;
pub use io::{CircuitIO, CircuitWithIO};

pub fn create_picus_backend<'b, L: LiftLike>(
    params: PicusParams,
) -> impl Backend<'b, PicusParams, PicusOutput<L>, F = L> {
    PicusBackend::initialize(params)
}

pub fn picus_codegen_with_params<L, C>(circuit: &C, params: PicusParams) -> Result<PicusOutput<L>>
where
    L: LiftLike,
    C: CircuitWithIO<L>,
{
    let backend = PicusBackend::initialize(params);
    backend.codegen_with_strat(circuit, &InlineConstraintsStrat)
}

pub fn picus_codegen<L, C>(circuit: &C) -> Result<PicusOutput<L>>
where
    L: LiftLike,
    C: CircuitWithIO<L>,
{
    picus_codegen_with_params(circuit, Default::default())
}
