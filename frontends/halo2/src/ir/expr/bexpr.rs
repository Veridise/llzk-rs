//! Structs for handling boolean expressions.

use super::super::{equivalency::EqvRelation, CmpOp};
use crate::{
    backend::lowering::{lowerable::LowerableExpr, ExprLowering},
    ir::{
        canon::canonicalize_constraint,
        expr::{Felt, IRAexpr},
    },
};
use anyhow::Result;
use std::{
    convert::identity,
    ops::{BitAnd, BitOr, Not},
};

/// Represents boolean expressions over some arithmetic expression type A.
pub enum IRBexpr<A> {
    /// Literal value for true.
    True,
    /// Literal value for false.
    False,
    /// Comparison operation of two inner arithmetic expressions.
    Cmp(CmpOp, A, A),
    /// Represents the conjunction of the inner expressions.
    And(Vec<IRBexpr<A>>),
    /// Represents the disjounction of the inner expressions.
    Or(Vec<IRBexpr<A>>),
    /// Represents the negation of the inner expression.
    Not(Box<IRBexpr<A>>),
}

impl<T> IRBexpr<T> {
    /// Transforms the inner expression into a different type.
    pub fn map<O>(self, f: &impl Fn(T) -> O) -> IRBexpr<O> {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => IRBexpr::Cmp(cmp_op, f(lhs), f(rhs)),
            IRBexpr::And(exprs) => IRBexpr::And(exprs.into_iter().map(|e| e.map(f)).collect()),
            IRBexpr::Or(exprs) => IRBexpr::Or(exprs.into_iter().map(|e| e.map(f)).collect()),
            IRBexpr::Not(expr) => IRBexpr::Not(Box::new(expr.map(f))),
            IRBexpr::True => IRBexpr::True,
            IRBexpr::False => IRBexpr::False,
        }
    }

    /// Transforms the inner expression into a different type without moving the struct.
    pub fn map_into<O>(&self, f: &impl Fn(&T) -> O) -> IRBexpr<O> {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => IRBexpr::Cmp(*cmp_op, f(lhs), f(rhs)),
            IRBexpr::And(exprs) => IRBexpr::And(exprs.iter().map(|e| e.map_into(f)).collect()),
            IRBexpr::Or(exprs) => IRBexpr::Or(exprs.iter().map(|e| e.map_into(f)).collect()),
            IRBexpr::Not(expr) => IRBexpr::Not(Box::new(expr.map_into(f))),
            IRBexpr::True => IRBexpr::True,
            IRBexpr::False => IRBexpr::False,
        }
    }

    /// Transforms the inner expression into a different type, potentially failing.
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
            IRBexpr::True => IRBexpr::True,
            IRBexpr::False => IRBexpr::False,
        })
    }

    /// Tries to transform the inner expression in place instead of returning a new expression.
    pub fn try_map_inplace(&mut self, f: &impl Fn(&mut T) -> Result<()>) -> Result<()> {
        match self {
            IRBexpr::Cmp(_, lhs, rhs) => {
                f(lhs)?;
                f(rhs)
            }
            IRBexpr::And(exprs) => {
                for expr in exprs {
                    expr.try_map_inplace(f)?;
                }
                Ok(())
            }
            IRBexpr::Or(exprs) => {
                for expr in exprs {
                    expr.try_map_inplace(f)?;
                }
                Ok(())
            }
            IRBexpr::Not(expr) => expr.try_map_inplace(f),
            IRBexpr::True => Ok(()),
            IRBexpr::False => Ok(()),
        }
    }
}

impl IRBexpr<IRAexpr> {
    /// Returns `Some(true)` or `Some(false)` if the expression is constant, `None` otherwise.
    pub fn const_value(&self) -> Option<bool> {
        match self {
            IRBexpr::True => Some(true),
            IRBexpr::False => Some(false),
            _ => None,
        }
    }

    /// Folds the expression if the values are constant.
    pub(crate) fn constant_fold(&mut self, prime: Felt) {
        fn collect_folds(exprs: &mut [IRBexpr<IRAexpr>], prime: Felt) -> Option<Vec<bool>> {
            exprs
                .iter_mut()
                .map(|e| {
                    e.constant_fold(prime);
                    e.const_value()
                })
                .collect()
        }
        match self {
            IRBexpr::True => {}
            IRBexpr::False => {}
            IRBexpr::Cmp(op, lhs, rhs) => {
                lhs.constant_fold(prime);
                rhs.constant_fold(prime);
                if let Some((lhs, rhs)) = lhs.const_value().zip(rhs.const_value()) {
                    *self = match op {
                        CmpOp::Eq => lhs == rhs,
                        CmpOp::Lt => lhs < rhs,
                        CmpOp::Le => lhs <= rhs,
                        CmpOp::Gt => lhs > rhs,
                        CmpOp::Ge => lhs >= rhs,
                        CmpOp::Ne => lhs != rhs,
                    }
                    .into()
                }
            }
            IRBexpr::And(exprs) => {
                if let Some(exprs) = collect_folds(exprs, prime) {
                    *self = exprs.into_iter().all(identity).into();
                }
            }
            IRBexpr::Or(exprs) => {
                if let Some(exprs) = collect_folds(exprs, prime) {
                    *self = exprs.into_iter().any(identity).into()
                }
            }
            IRBexpr::Not(expr) => {
                expr.constant_fold(prime);
                if let Some(b) = expr.const_value() {
                    *self = b.into();
                }
            }
        }
    }

    /// Matches the expressions against a series of known patterns and applies rewrites if able to.
    pub(crate) fn canonicalize(&mut self) {
        match self {
            IRBexpr::True => {}
            IRBexpr::False => {}
            IRBexpr::Cmp(op, lhs, rhs) => {
                if let Some((op, lhs, rhs)) = canonicalize_constraint(*op, lhs, rhs) {
                    *self = IRBexpr::Cmp(op, lhs, rhs);
                }
            }
            IRBexpr::And(exprs) => {
                for expr in exprs {
                    expr.canonicalize();
                }
            }
            IRBexpr::Or(exprs) => {
                for expr in exprs {
                    expr.canonicalize();
                }
            }
            IRBexpr::Not(expr) => {
                expr.canonicalize();
                match &**expr {
                    IRBexpr::True => {
                        *self = IRBexpr::False;
                    }
                    IRBexpr::False => {
                        *self = IRBexpr::True;
                    }
                    IRBexpr::Cmp(op, lhs, rhs) => {
                        *self = IRBexpr::Cmp(
                            match op {
                                CmpOp::Eq => CmpOp::Ne,
                                CmpOp::Lt => CmpOp::Ge,
                                CmpOp::Le => CmpOp::Gt,
                                CmpOp::Gt => CmpOp::Le,
                                CmpOp::Ge => CmpOp::Lt,
                                CmpOp::Ne => CmpOp::Eq,
                            },
                            lhs.clone(),
                            rhs.clone(),
                        );
                        self.canonicalize();
                    }
                    _ => {}
                }
            }
        }
    }
}

impl<T> From<bool> for IRBexpr<T> {
    fn from(value: bool) -> Self {
        if value {
            IRBexpr::True
        } else {
            IRBexpr::False
        }
    }
}

/// IRBexpr transilitively inherits any equivalence relation.
impl<L, R, E: EqvRelation<L, R>> EqvRelation<IRBexpr<L>, IRBexpr<R>> for E {
    /// Two boolean expressions are equivalent if they are structurally equal and their inner entities
    /// are equivalent.
    fn equivalent(lhs: &IRBexpr<L>, rhs: &IRBexpr<R>) -> bool {
        match (lhs, rhs) {
            (IRBexpr::Cmp(op1, lhs1, rhs1), IRBexpr::Cmp(op2, lhs2, rhs2)) => {
                op1 == op2 && E::equivalent(lhs1, lhs2) && E::equivalent(rhs1, rhs2)
            }
            (IRBexpr::And(lhs), IRBexpr::And(rhs)) => {
                <E as EqvRelation<Vec<IRBexpr<L>>, Vec<IRBexpr<R>>>>::equivalent(lhs, rhs)
            }
            (IRBexpr::Or(lhs), IRBexpr::Or(rhs)) => {
                <E as EqvRelation<Vec<IRBexpr<L>>, Vec<IRBexpr<R>>>>::equivalent(lhs, rhs)
            }
            (IRBexpr::Not(lhs), IRBexpr::Not(rhs)) => {
                <E as EqvRelation<Box<IRBexpr<L>>, Box<IRBexpr<R>>>>::equivalent(lhs, rhs)
            }
            _ => false,
        }
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
            IRBexpr::True => write!(f, "TRUE"),
            IRBexpr::False => write!(f, "FALSE"),
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
            IRBexpr::True => IRBexpr::True,
            IRBexpr::False => IRBexpr::False,
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
            (IRBexpr::True, IRBexpr::True) => true,
            (IRBexpr::False, IRBexpr::False) => false,
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
    A: LowerableExpr,
    L: ExprLowering + ?Sized,
{
    exprs
        .into_iter()
        .map(|e| e.lower(l))
        .reduce(|lhs, rhs| lhs.and_then(|lhs| rhs.and_then(|rhs| cb(l, &lhs, &rhs))))
        .ok_or_else(|| anyhow::anyhow!("Boolean expression with no elements"))
        .and_then(identity)
}

impl<F> IRBexpr<F> {}

impl<A: LowerableExpr> LowerableExpr for IRBexpr<A> {
    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering + ?Sized,
    {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => {
                let lhs = lhs.lower(l)?;
                let rhs = rhs.lower(l)?;
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
            IRBexpr::Not(expr) => expr.lower(l).and_then(|e| l.lower_not(&e)),
            IRBexpr::True => l.lower_true(),
            IRBexpr::False => l.lower_false(),
        }
    }
}
