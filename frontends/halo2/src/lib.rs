#![feature(iter_intersperse)]

use lookups::callbacks::LookupCallbacks;

#[cfg(feature = "lift-field-operations")]
mod arena;
pub(crate) mod backend;
pub mod driver;
mod error;
mod expressions;
mod gates;
mod halo2;
mod io;
pub mod ir;
pub mod lookups;
mod synthesis;
mod utils;
mod value;

use crate::halo2::{Advice, Circuit, Expression, Field, Instance, RegionIndex};
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

/// An implementation of this trait returns additional IR per region.
///
/// Temporary solution until the driver is refactored to work in 3 stages instead of the
/// current two.
pub trait IRInjectCallback<F: Field> {
    /// Returns IR that needs to be injected for the given region.
    ///
    /// Each expression is returned as a relative offset from the region's first row and the
    /// expression is lowered in the context of that particular region-row.
    ///
    /// If no IR needs to be injected for the given region return None.
    ///
    /// The call to inject must clean its internal resources to avoid emitting twice. The intended
    /// IR should be injected only once regardless of how many times the method is called for the
    /// same region.
    fn inject(
        &mut self,
        region: RegionIndex,
        start: usize,
    ) -> Option<crate::ir::stmt::IRStmt<(usize, Expression<F>)>>;
}

impl<F: Field> IRInjectCallback<F> for Option<&mut dyn IRInjectCallback<F>> {
    fn inject(
        &mut self,
        region: RegionIndex,
        start: usize,
    ) -> Option<crate::ir::stmt::IRStmt<(usize, Expression<F>)>> {
        match self {
            Some(injector) => injector.inject(region, start),
            None => None,
        }
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
