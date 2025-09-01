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
pub mod support;
mod synthesis;
#[cfg(test)]
mod test;
mod value;

use crate::halo2::{Advice, Circuit, Field, Instance};
#[cfg(feature = "lift-field-operations")]
pub use crate::ir::lift::{Lift, LiftLike};
pub use backend::{
    events::{
        BackendEventReceiver, BackendMessages, BackendResponse, EmitStmtsMessage, EventReceiver,
        EventSender, OwnedEventSender,
    },
    llzk::{LlzkBackend, LlzkOutput, LlzkParams, LlzkParamsBuilder},
    picus::{PicusBackend, PicusOutput, PicusParams, PicusParamsBuilder},
    Backend,
};
pub use error::to_plonk_error;
pub use field::LoweringField;
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
pub mod field {
    use crate::ir::lift::LiftLike;

    pub trait LoweringField: LiftLike {}

    impl<F: LiftLike> LoweringField for F {}
}

#[cfg(not(feature = "lift-field-operations"))]
pub mod field {
    use crate::halo2::PrimeField;

    pub trait LoweringField: PrimeField {}

    impl<F: PrimeField> LoweringField for F {}
}
