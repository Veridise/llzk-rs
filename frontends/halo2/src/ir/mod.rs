use crate::backend::func::FuncIO;

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
    ConstraintCall(String, Vec<T>, Vec<FuncIO>),
    Constraint(BinaryBoolOp, T, T),
    Comment(String),
    AssumeDeterministic(FuncIO),
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
            CircuitStmt::AssumeDeterministic(func_io) => CircuitStmt::AssumeDeterministic(*func_io),
        }
    }
}
