//! Error type.

use std::convert::Infallible;

use thiserror::Error;

/// IR error type.
#[derive(Error, Clone, Debug)]
pub enum Error {
    /// Happens while lowering [`IRBexpr`](crate::expr::IRBexpr) with no arguments (i.e. an empty
    /// `and` expression).
    #[error("Boolean expression with no elements")]
    EmptyBexpr,
    /// Happens while constant folding a [`IRBexpr`](crate::expr::IRBexpr) that folds into `false`.
    #[error("Detected {0} statement with predicate evaluating to 'false': {1}")]
    FoldedFalseStmt(&'static str, String),
}

impl From<Infallible> for Error {
    fn from(_value: Infallible) -> Self {
        unreachable!()
    }
}
