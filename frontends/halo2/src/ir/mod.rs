use anyhow::Result;

use crate::backend::{
    func::FuncIO,
    lowering::{Lowerable, Lowering, LoweringOutput},
};

pub mod lift;

pub use stmt::CmpOp as BinaryBoolOp;
pub mod expr;
mod stmt;

pub(crate) use stmt::chain_lowerable_stmts;
pub use stmt::CircuitStmt;
