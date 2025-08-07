use super::CmpOp;
use crate::backend::lowering::{lowerable::Lowerable, lowerable::LoweringOutput, Lowering};
use anyhow::Result;
use std::{
    convert::identity,
    ops::{BitAnd, BitOr, Not},
};

/// Represents boolean expressions over some arithmetic expression type A.
pub enum IRBexpr<A> {
    Cmp(CmpOp, A, A),
    And(Vec<IRBexpr<A>>),
    Or(Vec<IRBexpr<A>>),
    Not(Box<IRBexpr<A>>),
}

impl<T> IRBexpr<T> {
    pub fn map<O>(self, f: &impl Fn(T) -> O) -> IRBexpr<O> {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => IRBexpr::Cmp(cmp_op, f(lhs), f(rhs)),
            IRBexpr::And(exprs) => IRBexpr::And(exprs.into_iter().map(|e| e.map(f)).collect()),
            IRBexpr::Or(exprs) => IRBexpr::Or(exprs.into_iter().map(|e| e.map(f)).collect()),
            IRBexpr::Not(expr) => IRBexpr::Not(Box::new(expr.map(f))),
        }
    }

    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<IRBexpr<O>> {
        Ok(match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => IRBexpr::Cmp(cmp_op, f(lhs)?, f(rhs)?),
            IRBexpr::And(exprs) => IRBexpr::And(
                exprs
                    .into_iter()
                    .map(|e| e.try_map(f))
                    .collect::<Result<Vec<_>>>()?,
            ),
            IRBexpr::Or(exprs) => IRBexpr::Or(
                exprs
                    .into_iter()
                    .map(|e| e.try_map(f))
                    .collect::<Result<Vec<_>>>()?,
            ),
            IRBexpr::Not(expr) => IRBexpr::Not(Box::new(expr.try_map(f)?)),
        })
    }
}

fn concat<L, R, T>(lhs: L, rhs: R) -> Vec<IRBexpr<T>>
where
    L: IntoIterator<Item = IRBexpr<T>>,
    R: IntoIterator<Item = IRBexpr<T>>,
{
    lhs.into_iter().chain(rhs).collect()
}

impl<T> BitAnd for IRBexpr<T> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (IRBexpr::And(lhs), IRBexpr::And(rhs)) => IRBexpr::And(concat(lhs, rhs)),
            (lhs, IRBexpr::And(rhs)) => IRBexpr::And(concat([lhs], rhs)),
            (IRBexpr::And(lhs), rhs) => IRBexpr::And(concat(lhs, [rhs])),
            (lhs, rhs) => IRBexpr::And(vec![lhs, rhs]),
        }
    }
}

impl<T> BitOr for IRBexpr<T> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (IRBexpr::Or(lhs), IRBexpr::Or(rhs)) => IRBexpr::Or(concat(lhs, rhs)),
            (lhs, IRBexpr::Or(rhs)) => IRBexpr::Or(concat([lhs], rhs)),
            (IRBexpr::Or(lhs), rhs) => IRBexpr::Or(concat(lhs, [rhs])),
            (lhs, rhs) => IRBexpr::Or(vec![lhs, rhs]),
        }
    }
}

impl<T> Not for IRBexpr<T> {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            IRBexpr::Not(e) => *e,
            e => IRBexpr::Not(Box::new(e)),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for IRBexpr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => write!(f, "{lhs:?} {cmp_op} {rhs:?}",),
            IRBexpr::And(exprs) => write!(f, "AND {exprs:?}"),
            IRBexpr::Or(exprs) => write!(f, "OR {exprs:?}"),
            IRBexpr::Not(expr) => write!(f, "NOT {expr:?}"),
        }
    }
}

impl<T: Clone> Clone for IRBexpr<T> {
    fn clone(&self) -> Self {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => IRBexpr::Cmp(*cmp_op, lhs.clone(), rhs.clone()),
            IRBexpr::And(exprs) => IRBexpr::And(exprs.clone()),
            IRBexpr::Or(exprs) => IRBexpr::Or(exprs.clone()),
            IRBexpr::Not(expr) => IRBexpr::Not(expr.clone()),
        }
    }
}

impl<T: PartialEq> PartialEq for IRBexpr<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (IRBexpr::Cmp(op1, lhs1, rhs1), IRBexpr::Cmp(op2, lhs2, rhs2)) => {
                op1 == op2 && lhs1 == lhs2 && rhs1 == rhs2
            }
            (IRBexpr::And(lhs), IRBexpr::And(rhs)) => lhs == rhs,
            (IRBexpr::Or(lhs), IRBexpr::Or(rhs)) => lhs == rhs,
            (IRBexpr::Not(lhs), IRBexpr::Not(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

fn reduce_bool_expr<A, L>(
    exprs: impl IntoIterator<Item = IRBexpr<A>>,
    l: &L,
    cb: impl Fn(&L, &L::CellOutput, &L::CellOutput) -> Result<L::CellOutput>,
) -> Result<L::CellOutput>
where
    A: Lowerable<F = L::F>,
    L: Lowering + ?Sized,
{
    exprs
        .into_iter()
        .map(|e| l.lower_value(e))
        .reduce(|lhs, rhs| lhs.and_then(|lhs| rhs.and_then(|rhs| cb(l, &lhs, &rhs))))
        .ok_or_else(|| anyhow::anyhow!("Boolean expression with no elements"))
        .and_then(identity)
}

impl<F> IRBexpr<F> {}

impl<A: Lowerable> Lowerable for IRBexpr<A> {
    type F = A::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => {
                let lhs = l.lower_value(lhs)?;
                let rhs = l.lower_value(rhs)?;
                match cmp_op {
                    CmpOp::Eq => l.lower_eq(&lhs, &rhs),
                    CmpOp::Lt => l.lower_lt(&lhs, &rhs),
                    CmpOp::Le => l.lower_le(&lhs, &rhs),
                    CmpOp::Gt => l.lower_gt(&lhs, &rhs),
                    CmpOp::Ge => l.lower_ge(&lhs, &rhs),
                    CmpOp::Ne => l.lower_ne(&lhs, &rhs),
                }
            }
            IRBexpr::And(exprs) => reduce_bool_expr(exprs, l, L::lower_and),
            IRBexpr::Or(exprs) => reduce_bool_expr(exprs, l, L::lower_or),
            IRBexpr::Not(expr) => l.lower_value(expr).and_then(|e| l.lower_not(&e)),
        }
    }
}
