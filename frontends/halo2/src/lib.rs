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
pub mod info_traits;
mod io;
pub mod ir;
pub mod lookups;
mod resolvers;
mod synthesis;
pub mod temps;
mod utils;
mod value;

use crate::{
    halo2::Field,
    info_traits::ConstraintSystemInfo,
    io::{AdviceIO, InstanceIO},
};
#[cfg(feature = "llzk-backend")]
pub use backend::llzk::{
    LlzkOutput,
    params::{LlzkParams, LlzkParamsBuilder},
};
#[cfg(feature = "picus-backend")]
pub use backend::picus::{
    PicusOutput,
    params::{PicusParams, PicusParamsBuilder},
};
pub use error::to_plonk_error;
pub use expressions::ExpressionInRow;
pub use gates::{GateCallbacks, GateRewritePattern, GateScope, RewriteError, RewriteOutput};
pub use io::CircuitIO;
pub use lookups::callbacks::LookupCallbacks;
pub use synthesis::Synthesizer;

/// Implementations of this trait define how a circuit is synthesized.
///
/// Serves as a bridge to the Halo2 circuit synthesis process that allows disconnecting the types
/// defined in this crate with the types defined by Halo2. Since many Halo2 based projects fork the
/// library this trait allows for swapping the concrete implementation of Halo2 without having to
/// change the codebase of this crate.
///
/// # Note
///
/// At the time of writing removing the dependency on Halo2 is a work in progress and some types in this crate still
/// depend on types defined by Halo2.
pub trait CircuitSynthesis<F: Field> {
    /// The type of the circuit.
    type Circuit;
    /// Should be the same type as the circuit config.
    type Config;
    /// Type of the constraint system.
    type CS: ConstraintSystemInfo<F> + Default + 'static;
    /// Error type for synthesis.
    type Error: std::error::Error + Sync + Send + 'static;

    /// Returns a reference to the circuit.
    fn circuit(&self) -> &Self::Circuit;

    /// Creates the configuration of the circuit.
    fn configure(cs: &mut Self::CS) -> Self::Config;

    /// Returns the advice cells that are part of the inputs and outputs of the circuit.
    fn advice_io(config: &Self::Config) -> anyhow::Result<AdviceIO>;

    /// Returns the instance cells that are part of the inputs and outputs of the circuit.
    fn instance_io(config: &Self::Config) -> anyhow::Result<InstanceIO>;

    /// This callback requests the client to fill out the [`Synthesizer`] with the synthesis
    /// information about the circuit.
    ///
    /// Has a default implementation as part of the halo2 removal process. This method will be a
    /// required method in the final version.
    fn synthesize(
        circuit: &Self::Circuit,
        config: Self::Config,
        synthesizer: &mut Synthesizer<F>,
        cs: &Self::CS,
    ) -> Result<(), Self::Error>;
}
