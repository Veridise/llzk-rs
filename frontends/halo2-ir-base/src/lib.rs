#![doc = include_str!("../README.md")]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

pub mod cmp;
pub mod felt;
pub mod func;
//pub mod temps;

/// Equivalence relation on symbolic equivalence.
///
/// Symbolic in this context means that when comparing
/// entities information that does not affect the semantics
/// of what the entities are expression is ignored.
#[derive(Debug)]
pub struct SymbolicEqv;
