//! Error type.

use std::sync::Arc;

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
    Lowering(Arc<dyn std::error::Error>),
    /// Error emitted by implementations of [`Lowering`](crate::Lowering) or
    /// [`ExprLowering`](crate::ExprLowering).
    ///
    /// Use [`backend_err!`] to easily create this kind of error.
    #[error("Backend error")]
    Backend(Arc<dyn std::error::Error>),
}

unsafe impl Send for Error {}
unsafe impl Sync for Error {}

/// Convenience macro for creating [`Error::Lowering`] type of errors.
#[macro_export]
macro_rules! lowering_err {
    ($err:expr) => {
        $crate::error::Error::Lowering(std::sync::Arc::new($err))
    };
}

/// Convenience macro for creating [`Error::Backend`] type of errors.
#[macro_export]
macro_rules! backend_err {
    ($err:expr) => {
        $crate::error::Error::Backend(std::sync::Arc::new($err))
    };
}

/// Convenience macro for creating [`Error::Backend`] type of errors and immediately returning.
#[macro_export]
macro_rules! bail_backend {
    ($err:expr) => {{
        return Err($crate::error::Error::Backend(std::sync::Arc::new($err)));
    }};
}
