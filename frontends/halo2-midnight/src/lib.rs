#![doc = include_str!("../README.md")]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

pub mod circuit;
mod macros;
pub mod plonk;
pub mod synthesizer;

/// Trait implemented by types wrapped by a newtype in this crate.
pub trait Wrapped: Sized {
    /// Wrapper type.
    type Wrapper;

    /// Wraps the value into its wrapper.
    fn wrap(self) -> Self::Wrapper;
}
