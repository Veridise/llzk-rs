//! Adaptor traits that clients need to implement for communicating with the [`crate::driver::Driver`].

use ff::Field;

use crate::{
    expressions::{EvaluableExpr, ExprBuilder, ExpressionInfo},
    lookups::LookupData,
};

/// Trait for querying information about the constraint system derived during configuration.
pub trait ConstraintSystemInfo<F: Field> {
    /// Type for polynomial expressions.
    type Polynomial: EvaluableExpr<F> + Clone + ExpressionInfo + ExprBuilder<F>;

    /// Returns the list of gates defined in the system.
    fn gates(&self) -> Vec<&dyn GateInfo<Self::Polynomial>>;

    /// Returns a list with data about the lookups defined in the system.
    fn lookups<'cs>(&'cs self) -> Vec<LookupData<'cs, Self::Polynomial>>;
}

/// Trait for querying information about the a gate in the constraint system.
pub trait GateInfo<P> {
    /// Returns the name of the gate.
    fn name(&self) -> &str;

    /// Returns the list of polynomials that make up the gate.
    fn polynomials(&self) -> &[P];
}
