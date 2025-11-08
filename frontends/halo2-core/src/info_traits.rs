//! Adaptor traits that clients need to implement in order to integrate with the frontend.

use ff::Field;

use crate::{
    expressions::{EvaluableExpr, ExprBuilder, ExpressionInfo},
    lookups::LookupData,
    query::QueryKind,
    table::{Cell, Rotation},
};

/// Trait for querying information about the constraint system derived during configuration.
pub trait ConstraintSystemInfo<F: Field> {
    /// Type for polynomial expressions.
    type Polynomial: EvaluableExpr<F> + Clone + ExpressionInfo + ExprBuilder<F>;

    /// Notifies the constraint system that the circuit has completed synthesis.
    fn synthesis_completed(&mut self) {}

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

/// Trait for retrieving information about cell queries.
pub trait QueryInfo {
    /// The kind of query this implementation provides information about.
    type Kind: QueryKind;

    /// Returns the rotation offset.
    fn rotation(&self) -> Rotation;

    /// Returns the index of the column the queried cell belongs to.
    fn column_index(&self) -> usize;
}

/// Trait for constructing queries as expressions.
pub trait CreateQuery<E> {
    /// Constructs an expression representing the query.
    fn query_expr(index: usize, at: Rotation) -> E;
}

/// Trait for retrieving information about a selector.
pub trait SelectorInfo {
    /// Returns the identifier of the selector.
    fn id(&self) -> usize;
}

/// Trait for retrieving information about a challenge.
pub trait ChallengeInfo {
    /// Returns the index of the challenge.
    fn index(&self) -> usize;

    /// Returns the phase number of the challenge.
    fn phase(&self) -> u8;
}

/// Trait for retrieving information about group annotations.
pub trait GroupInfo {
    /// Returns the inputs of the group.
    fn inputs(&self) -> impl Iterator<Item = Cell> + '_;

    /// Returns the outputs of the group.
    fn outputs(&self) -> impl Iterator<Item = Cell> + '_;
}
