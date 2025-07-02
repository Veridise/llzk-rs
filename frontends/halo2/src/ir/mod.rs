use crate::halo2::Value;

pub mod lift;

/// IR for operations that occur in the main circuit.
pub enum CircuitStmt<T> {
    ConstraintCall(String, Vec<Value<T>>, Vec<Value<T>>),
    EqConstraint(Value<T>, Value<T>),
    Comment(String),
}

pub type CircuitStmts<T> = Vec<CircuitStmt<T>>;
