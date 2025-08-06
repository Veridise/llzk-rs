use std::{convert::identity, marker::PhantomData};

use anyhow::Result;

use crate::backend::{
    func::FuncIO,
    lowering::{Lowerable, Lowering, LoweringOutput},
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    Ne,
}

impl std::fmt::Display for CmpOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CmpOp::Eq => "==",
                CmpOp::Lt => "<",
                CmpOp::Le => "<=",
                CmpOp::Gt => ">",
                CmpOp::Ge => ">=",
                CmpOp::Ne => "!=",
            }
        )
    }
}

pub struct Constraint<T> {
    op: CmpOp,
    lhs: T,
    rhs: T,
}

impl<T> Constraint<T> {
    pub fn new(op: CmpOp, lhs: T, rhs: T) -> Self {
        Self { op, lhs, rhs }
    }
    pub fn map<O>(self, f: &impl Fn(T) -> O) -> Constraint<O> {
        Constraint::new(self.op, f(self.lhs), f(self.rhs))
    }

    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<Constraint<O>> {
        Ok(Constraint::new(self.op, f(self.lhs)?, f(self.rhs)?))
    }
}

impl<T: Lowerable> Lowerable for Constraint<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        let lhs = l.lower_value(self.lhs)?;
        let rhs = l.lower_value(self.rhs)?;
        l.generate_constraint(self.op, &lhs, &rhs)
    }
}

impl<T: Clone> Clone for Constraint<T> {
    fn clone(&self) -> Self {
        Self {
            op: self.op,
            lhs: self.lhs.clone(),
            rhs: self.rhs.clone(),
        }
    }
}

impl<T: PartialEq> PartialEq for Constraint<T> {
    fn eq(&self, other: &Self) -> bool {
        self.op == other.op && self.lhs == other.lhs && self.rhs == other.rhs
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Constraint<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {} {:?}", self.lhs, self.op, self.rhs)
    }
}
