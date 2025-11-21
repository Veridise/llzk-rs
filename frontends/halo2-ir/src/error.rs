//! Error type.

use thiserror::Error;

use crate::expr::{IRAexpr, IRBexpr};

/// IR error type.
#[derive(Error, Clone, Debug)]
pub enum Error {
    /// Happens while lowering [`IRBexpr`] with no arguments (i.e. an empty
    /// `and` expression).
    #[error("Boolean expression with no elements")]
    EmptyBexpr,
    /// Happens while constant folding a [`IRBexpr`] that folds into `false`.
    #[error("Detected {0} statement with predicate evaluating to 'false': {1:#?}")]
    FoldedFalseStmt(&'static str, IRBexpr<IRAexpr>),
}
