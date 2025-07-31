use anyhow::Result;

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
    Assert(T),
    Seq(Vec<CircuitStmt<T>>),
}

impl<T> CircuitStmt<T> {
    pub fn reduce<O: Default>(
        self,
        call: &impl Fn(String, Vec<T>, Vec<FuncIO>) -> Result<O>,
        constraint: &impl Fn(BinaryBoolOp, T, T) -> Result<O>,
        comment: &impl Fn(String) -> Result<O>,
        assume_deterministic: &impl Fn(FuncIO) -> Result<O>,
        assert: &impl Fn(T) -> Result<O>,
        join: &impl Fn(O, O) -> O,
    ) -> Result<O> {
        match self {
            CircuitStmt::ConstraintCall(module, inputs, outputs) => call(module, inputs, outputs),
            CircuitStmt::Constraint(op, lhs, rhs) => constraint(op, lhs, rhs),
            CircuitStmt::Comment(s) => comment(s),
            CircuitStmt::AssumeDeterministic(func_io) => assume_deterministic(func_io),
            CircuitStmt::Assert(expr) => assert(expr),
            CircuitStmt::Seq(stmts) => stmts.into_iter().try_fold(Default::default(), |o, s| {
                Ok(join(
                    o,
                    s.reduce(
                        call,
                        constraint,
                        comment,
                        assume_deterministic,
                        assert,
                        join,
                    )?,
                ))
            }),
        }
    }

    pub fn map<O>(
        &self,
        call: &impl Fn(&str, &[T], &[FuncIO]) -> Result<(String, Vec<O>, Vec<FuncIO>)>,
        constraint: &impl Fn(BinaryBoolOp, &T, &T) -> Result<(BinaryBoolOp, O, O)>,
        comment: &impl Fn(&str) -> Result<String>,
        assume_deterministic: &impl Fn(FuncIO) -> Result<FuncIO>,
        assert: &impl Fn(&T) -> Result<O>,
    ) -> Result<CircuitStmt<O>> {
        match self {
            CircuitStmt::ConstraintCall(module, inputs, outputs) => {
                let (module, inputs, outputs) = call(module, inputs, outputs)?;
                Ok(CircuitStmt::ConstraintCall(module, inputs, outputs))
            }
            CircuitStmt::Constraint(op, lhs, rhs) => {
                let (op, lhs, rhs) = constraint(*op, lhs, rhs)?;
                Ok(CircuitStmt::Constraint(op, lhs, rhs))
            }
            CircuitStmt::Comment(s) => comment(s).map(CircuitStmt::Comment),
            CircuitStmt::AssumeDeterministic(func_io) => {
                assume_deterministic(*func_io).map(CircuitStmt::AssumeDeterministic)
            }
            CircuitStmt::Assert(expr) => assert(expr).map(CircuitStmt::Assert),
            CircuitStmt::Seq(stmts) => Ok(CircuitStmt::Seq(
                stmts
                    .iter()
                    .map(|e| e.map(call, constraint, comment, assume_deterministic, assert))
                    .collect::<Result<Vec<_>>>()?,
            )),
        }
    }
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
            CircuitStmt::Assert(e) => CircuitStmt::Assert(e.clone()),
            CircuitStmt::Seq(stmts) => CircuitStmt::Seq(stmts.clone()),
        }
    }
}
