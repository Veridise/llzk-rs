#![feature(iter_intersperse)]

use crate::halo2::PrimeField;
use anyhow::Result;
use backend::codegen::strats::inline::InlineConstraintsStrat;
use lookups::callbacks::LookupCallbacks;

mod arena;
pub(crate) mod backend;
mod error;
mod expressions;
mod gates;
mod halo2;
mod io;
pub mod ir;
pub mod lookups;
mod synthesis;
#[cfg(test)]
mod test;
mod value;

use crate::halo2::{Advice, Circuit, Field, Instance};
#[cfg(feature = "lift-field-operations")]
pub use crate::ir::lift::{Lift, LiftLike};
pub use backend::events::{
    BackendEventReceiver, BackendMessages, BackendResponse, EmitStmtsMessage, EventReceiver,
    EventSender, OwnedEventSender,
};
pub use backend::picus::PicusBackend;
pub use backend::picus::PicusEventReceiver;
pub use backend::picus::PicusOutput;
pub use backend::picus::PicusParams;
pub use backend::picus::PicusParamsBuilder;
pub use backend::Backend;
pub use error::to_plonk_error;
pub use gates::{GateCallbacks, GateRewritePattern, GateScope, RewriteError, RewriteOutput};
pub use io::CircuitIO;
pub use synthesis::regions::RegionRowLike;

/// Defines, for a given circuit, a set of callbacks with information
/// required by the LLZK codegen module to construct the IR representation of the
/// circuit.
pub trait CircuitCallbacks<F: Field, C: Circuit<F>> {
    fn advice_io(config: &C::Config) -> CircuitIO<Advice>;

    fn instance_io(config: &C::Config) -> CircuitIO<Instance>;

    fn lookup_callbacks() -> Option<Box<dyn LookupCallbacks<F>>> {
        None
    }

    fn gate_callbacks() -> Option<Box<dyn GateCallbacks<F>>> {
        None
    }
}

#[cfg(feature = "lift-field-operations")]
pub fn create_picus_backend<L>(params: PicusParams) -> PicusBackend<L>
where
    L: LiftLike,
{
    PicusBackend::initialize(params)
}

#[cfg(feature = "lift-field-operations")]
pub fn picus_codegen_with_params<L, C>(circuit: &C, params: PicusParams) -> Result<PicusOutput<L>>
where
    L: LiftLike,
    C: Circuit<L> + CircuitCallbacks<L, C>,
{
    let backend = PicusBackend::initialize(params);
    backend.codegen_with_strat::<C, C, InlineConstraintsStrat>(circuit)
}

#[cfg(feature = "lift-field-operations")]
pub fn picus_codegen<L, C>(circuit: &C) -> Result<PicusOutput<L>>
where
    L: LiftLike,
    C: Circuit<L> + CircuitCallbacks<L, C>,
{
    picus_codegen_with_params(circuit, Default::default())
}

#[cfg(not(feature = "lift-field-operations"))]
pub fn create_picus_backend<L>(params: PicusParams) -> PicusBackend<L>
where
    L: PrimeField,
{
    PicusBackend::initialize(params)
}

#[cfg(not(feature = "lift-field-operations"))]
pub fn picus_codegen_with_params<L, C>(circuit: &C, params: PicusParams) -> Result<PicusOutput<L>>
where
    L: PrimeField,
    C: Circuit<L> + CircuitCallbacks<L, C>,
{
    let backend = PicusBackend::initialize(params);
    backend.codegen_with_strat::<C, C, InlineConstraintsStrat>(circuit)
}

#[cfg(not(feature = "lift-field-operations"))]
pub fn picus_codegen<L, C>(circuit: &C) -> Result<PicusOutput<L>>
where
    L: PrimeField,
    C: Circuit<L> + CircuitCallbacks<L, C>,
{
    picus_codegen_with_params(circuit, Default::default())
}
