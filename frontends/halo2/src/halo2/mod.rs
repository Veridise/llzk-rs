//! Opaque module that exposes the correct halo2 library based on the implementation selected via
//! feature flags.

#[cfg(not(feature = "midnight"))]
pub use halo2curves::bn256;

#[cfg(feature = "axiom")]
mod axiom;
#[cfg(feature = "midnight")]
mod midnight;
#[cfg(feature = "pse")]
mod pse;
#[cfg(feature = "pse-v1")]
mod pse_v1;
#[cfg(feature = "scroll")]
mod scroll;
#[cfg(feature = "zcash")]
mod zcash;

#[cfg(feature = "axiom")]
pub use axiom::*;
#[cfg(feature = "midnight")]
pub use midnight::*;
#[cfg(feature = "pse")]
pub use pse::*;
#[cfg(feature = "pse-v1")]
pub use pse_v1::*;
#[cfg(feature = "scroll")]
pub use scroll::*;
#[cfg(feature = "zcash")]
pub use zcash::*;

// Conditionally require `Group` based on the presence of a feature flag
#[cfg(feature = "pse-v1")]
pub trait CodegenField: Field + Group {}
#[cfg(not(feature = "pse-v1"))]
pub trait CodegenField: Field {}
