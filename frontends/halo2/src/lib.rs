//! LLZK frontend for the Halo2 framework.

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

pub(crate) mod backend;
pub mod driver;
mod error;
mod expressions;
mod gates;
mod halo2;
mod io;
pub mod ir;
pub mod lookups;
mod resolvers;
mod synthesis;
mod utils;
mod value;

use crate::{
    halo2::{Circuit, Field},
    io::{AdviceIO, InstanceIO},
};
pub use backend::{
    llzk::{
        params::{LlzkParams, LlzkParamsBuilder},
        LlzkOutput,
    },
    picus::{
        params::{PicusParams, PicusParamsBuilder},
        PicusOutput,
    },
};
pub use error::to_plonk_error;
pub use gates::{GateCallbacks, GateRewritePattern, GateScope, RewriteError, RewriteOutput};
pub use io::CircuitIO;
pub use lookups::callbacks::LookupCallbacks;

/// Defines, for a given circuit, a set of callbacks with information
/// required by the LLZK codegen module to construct the IR representation of the
/// circuit.
pub trait CircuitCallbacks<F: Field>: Circuit<F> {
    /// Returns the advice cells that are part of the inputs and outputs of the circuit.
    fn advice_io(config: &Self::Config) -> AdviceIO;

    /// Returns the instance cells that are part of the inputs and outputs of the circuit.
    fn instance_io(config: &Self::Config) -> InstanceIO;
}
