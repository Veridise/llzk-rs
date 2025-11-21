//! Error type.

use thiserror::Error;

/// Lowering error type.
#[derive(Error, Debug)]
pub enum Error {
    /// Happens when [`Lowering::checked_generate_constraint`](crate::Lowering::checked_generate_constraint) fails
    /// because the constraint was not generated.
    #[error("Last constraint was not generated!")]
    LastConstraintNotGenerated,
    /// Error emitted by implementations of [`LowerableStmt`](crate::lowerable::LowerableStmt) or
    /// [`LowerableExpr`](crate::lowerable::LowerableExpr).
    ///
    /// Use [`lowering_err!`] to easily create this kind of error.
    #[error("Lowering error")]
    Lowering(Box<dyn std::error::Error>),
}

/// Convenience macro for creating [`Error::Lowering`] type of errors.
#[macro_export]
macro_rules! lowering_err {
    ($err:expr) => {
        $crate::error::Error::Lowering(Box::new($err))
    };
}
