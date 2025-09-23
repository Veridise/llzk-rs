//! Tiny optimization IR.

use egg::Id;

use crate::{
    backend::func::FuncIO,
    halo2::Challenge,
    ir::{expr::Felt, CmpOp},
};

#[allow(dead_code)]
pub enum Aexpr {
    Lit(Felt),
    Var(FuncIO),
    Chall(Challenge),
    Neg(Id),
    Sum([Id; 2]),
    Product([Id; 2]),
}

#[allow(dead_code)]
pub enum Bexpr {
    Lit(bool),
    Cmp(CmpOp, Aexpr, Aexpr),
    Not(Id),
    And([Id; 2]),
    Or([Id; 2]),
}

#[allow(dead_code)]
pub enum Stmt {
    Constr(CmpOp, Aexpr, Aexpr),
    Assert(Bexpr),
    Seq([Id; 2]),
}
