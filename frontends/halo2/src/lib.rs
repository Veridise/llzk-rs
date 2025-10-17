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
pub mod temps;
mod utils;
mod value;

use crate::{
    halo2::{Circuit, Field},
    io::{AdviceIO, InstanceIO},
    synthesis::{Synthesizer, SynthesizerAssignment},
};
pub use backend::{
    llzk::{
        LlzkOutput,
        params::{LlzkParams, LlzkParamsBuilder},
    },
    picus::{
        PicusOutput,
        params::{PicusParams, PicusParamsBuilder},
    },
};
pub use error::to_plonk_error;
pub use expressions::ExpressionInRow;
pub use gates::{GateCallbacks, GateRewritePattern, GateScope, RewriteError, RewriteOutput};
pub use io::CircuitIO;
pub use lookups::callbacks::LookupCallbacks;

/// Defines, for a given circuit, a set of callbacks with information
/// required by the LLZK codegen module to construct the IR representation of the
/// circuit.
pub trait CircuitCallbacks<F: Field> {
    /// The type of the circuit.
    type Circuit: Circuit<F, Config = Self::Config>;
    /// Should be the same type as the circuit config.
    type Config;

    /// Returns the advice cells that are part of the inputs and outputs of the circuit.
    fn advice_io(config: &Self::Config) -> AdviceIO;

    /// Returns the instance cells that are part of the inputs and outputs of the circuit.
    fn instance_io(config: &Self::Config) -> InstanceIO;

    /// This callback requests the client to fill out the [`Synthesizer`] with the synthesis
    /// information about the circuit.
    ///
    /// Has a default implementation as part of the halo2 removal process. This method will be a
    /// required method in the final version.
    fn synthesize(
        circuit: &Self::Circuit,
        config: Self::Config,
        synthesizer: &mut Synthesizer<F>,
    ) -> Result<(), crate::halo2::Error> {
        let mut assign = SynthesizerAssignment::new(synthesizer);
        assign
            .synthesize(circuit, config)
            .map_err(|err| to_plonk_error(err))
    }
}
