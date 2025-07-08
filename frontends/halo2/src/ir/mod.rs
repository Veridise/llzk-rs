use crate::halo2::Value;

pub mod lift;

#[derive(Copy, Clone)]
pub enum BinaryBoolOp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    Ne,
}

/// IR for operations that occur in the main circuit.
pub enum CircuitStmt<T> {
    ConstraintCall(String, Vec<T>, Vec<T>),
    Constraint(BinaryBoolOp, T, T),
    Comment(String),
}

impl<T: Clone> Clone for CircuitStmt<T> {
    fn clone(&self) -> Self {
        match self {
            CircuitStmt::ConstraintCall(callee, inputs, outputs) => {
                CircuitStmt::ConstraintCall(callee.clone(), inputs.clone(), outputs.clone())
            }
            CircuitStmt::Constraint(op, lhs, rhs) => {
                CircuitStmt::Constraint(*op, lhs.clone(), rhs.clone())
            }
            CircuitStmt::Comment(c) => CircuitStmt::Comment(c.clone()),
        }
    }
}
