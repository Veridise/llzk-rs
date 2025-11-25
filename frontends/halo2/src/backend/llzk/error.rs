use std::sync::Arc;

use haloumi_ir_base::func::FieldId;
use llzk::prelude::Type;
use thiserror::Error;

/// General error type.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Struct is missing output #{0}")]
    MissingOutput(FieldId),
    #[error("Constrain function is missing!")]
    MissingConstrainFunc,
    #[error("Constraint function region is missing a block")]
    MissingBlock,
    #[error("Constraint function is missing a terminator")]
    MissingTerminator,
    #[error("LLZK error: {0}")]
    Llzk(#[from] llzk::error::Error),
    #[error("MLIR error: {0}")]
    Mlir(#[from] melior::Error),
}

impl From<Error> for haloumi_lowering::error::Error {
    fn from(value: Error) -> Self {
        Self::Backend(Arc::new(value))
    }
}

#[derive(Debug, Error)]
#[error("Was expecting type {expected} but got type {actual}")]
pub struct UnexpectedTypeError {
    expected: String,
    actual: String,
}

impl UnexpectedTypeError {
    pub fn new(expected: Type<'_>, actual: Type<'_>) -> Self {
        Self {
            expected: format!("{expected}"),
            actual: format!("{actual}"),
        }
    }
    pub fn with_context(self, msg: impl ToString) -> ContextualizedError<Self> {
        ContextualizedError {
            msg: msg.to_string(),
            error: self,
        }
    }
}

impl From<UnexpectedTypeError> for haloumi_lowering::error::Error {
    fn from(value: UnexpectedTypeError) -> Self {
        Self::Backend(Arc::new(value))
    }
}
#[derive(Debug, Error)]
#[error("{msg}: {error}")]
pub struct ContextualizedError<E: std::error::Error> {
    msg: String,
    #[source]
    error: E,
}

impl<E: std::error::Error + 'static> From<ContextualizedError<E>>
    for haloumi_lowering::error::Error
{
    fn from(value: ContextualizedError<E>) -> Self {
        Self::Backend(Arc::new(value))
    }
}
