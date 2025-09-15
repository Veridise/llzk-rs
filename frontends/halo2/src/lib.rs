use lookups::callbacks::LookupCallbacks;

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

use crate::halo2::{Circuit, Field};
pub use backend::{
    llzk::params::{LlzkParams, LlzkParamsBuilder},
    picus::params::{PicusParams, PicusParamsBuilder},
};
pub use error::to_plonk_error;
pub use gates::{GateCallbacks, GateRewritePattern, GateScope, RewriteError, RewriteOutput};
pub use io::CircuitIO;
pub use synthesis::regions::RegionRowLike;

/// Defines, for a given circuit, a set of callbacks with information
/// required by the LLZK codegen module to construct the IR representation of the
/// circuit.
pub trait CircuitCallbacks<F: Field, C: Circuit<F>> {
    fn advice_io(config: &C::Config) -> crate::io::AdviceIO;

    fn instance_io(config: &C::Config) -> crate::io::InstanceIO;

    fn lookup_callbacks() -> Option<Box<dyn LookupCallbacks<F>>> {
        None
    }

    fn gate_callbacks() -> Option<Box<dyn GateCallbacks<F>>> {
        None
    }
}
