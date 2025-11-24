//! Structs for handling boolean expressions.

use crate::error::Error;
use crate::{canon::canonicalize_constraint, expr::IRAexpr};
use eqv::{EqvRelation, equiv};
use haloumi_ir_base::SymbolicEqv;
//use anyhow::Result;
use haloumi_ir_base::cmp::CmpOp;
use haloumi_ir_base::felt::Felt;
use haloumi_lowering::lowering_err;
use haloumi_lowering::{ExprLowering, lowerable::LowerableExpr};
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
    /// Declares that the inner arithmetic expression needs to be proven deterministic
    Det(A),
    /// Logical implication operator.
    Implies(Box<IRBexpr<A>>, Box<IRBexpr<A>>),
    /// Logical double-implication operator.
    Iff(Box<IRBexpr<A>>, Box<IRBexpr<A>>),
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
            IRBexpr::Det(expr) => IRBexpr::Det(f(expr)),
            IRBexpr::Implies(lhs, rhs) => {
                IRBexpr::Implies(Box::new(lhs.map(f)), Box::new(rhs.map(f)))
            }
            IRBexpr::Iff(lhs, rhs) => IRBexpr::Iff(Box::new(lhs.map(f)), Box::new(rhs.map(f))),
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
            IRBexpr::Det(expr) => IRBexpr::Det(f(expr)),
            IRBexpr::Implies(lhs, rhs) => {
                IRBexpr::Implies(Box::new(lhs.map_into(f)), Box::new(rhs.map_into(f)))
            }
            IRBexpr::Iff(lhs, rhs) => {
                IRBexpr::Iff(Box::new(lhs.map_into(f)), Box::new(rhs.map_into(f)))
            }
        }
    }

    /// Transforms the inner expression into a different type, potentially failing.
    pub fn try_map<O, E>(self, f: &impl Fn(T) -> Result<O, E>) -> Result<IRBexpr<O>, E> {
        Ok(match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => IRBexpr::Cmp(cmp_op, f(lhs)?, f(rhs)?),
            IRBexpr::And(exprs) => IRBexpr::And(
                exprs
                    .into_iter()
                    .map(|e| e.try_map(f))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            IRBexpr::Or(exprs) => IRBexpr::Or(
                exprs
                    .into_iter()
                    .map(|e| e.try_map(f))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            IRBexpr::Not(expr) => IRBexpr::Not(Box::new(expr.try_map(f)?)),
            IRBexpr::True => IRBexpr::True,
            IRBexpr::False => IRBexpr::False,
            IRBexpr::Det(expr) => IRBexpr::Det(f(expr)?),
            IRBexpr::Implies(lhs, rhs) => {
                IRBexpr::Implies(Box::new(lhs.try_map(f)?), Box::new(rhs.try_map(f)?))
            }
            IRBexpr::Iff(lhs, rhs) => {
                IRBexpr::Iff(Box::new(lhs.try_map(f)?), Box::new(rhs.try_map(f)?))
            }
        })
    }

    /// Tries to transform the inner expression in place instead of returning a new expression.
    pub fn try_map_inplace<E>(&mut self, f: &impl Fn(&mut T) -> Result<(), E>) -> Result<(), E> {
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
            IRBexpr::Det(expr) => f(expr),
            IRBexpr::Implies(lhs, rhs) => {
                lhs.try_map_inplace(f)?;
                rhs.try_map_inplace(f)
            }
            IRBexpr::Iff(lhs, rhs) => {
                lhs.try_map_inplace(f)?;
                rhs.try_map_inplace(f)
            }
        }
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Eq`] between two expressions.
    pub fn eq(lhs: T, rhs: T) -> Self {
        Self::Cmp(CmpOp::Eq, lhs, rhs)
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Lt`] between two expressions.
    pub fn lt(lhs: T, rhs: T) -> Self {
        Self::Cmp(CmpOp::Lt, lhs, rhs)
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Le`] between two expressions.
    pub fn le(lhs: T, rhs: T) -> Self {
        Self::Cmp(CmpOp::Le, lhs, rhs)
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Gt`] between two expressions.
    pub fn gt(lhs: T, rhs: T) -> Self {
        Self::Cmp(CmpOp::Gt, lhs, rhs)
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Ge`] between two expressions.
    pub fn ge(lhs: T, rhs: T) -> Self {
        Self::Cmp(CmpOp::Ge, lhs, rhs)
    }

    #[inline]
    /// Creates an implication expression.
    pub fn implies(lhs: Self, rhs: Self) -> Self {
        Self::Implies(Box::new(lhs), Box::new(rhs))
    }

    /// Maps the statement's inner type to a tuple with the passed value.
    pub fn with<O>(self, other: O) -> IRBexpr<(O, T)>
    where
        O: Clone,
    {
        self.map(&|t| (other.clone(), t))
    }

    /// Maps the statement's inner type to a tuple with the result of the closure.
    pub fn with_fn<O>(self, other: impl Fn() -> O) -> IRBexpr<(O, T)> {
        self.map(&|t| (other(), t))
    }
}

struct LogLine {
    before: Option<String>,
    ident: usize,
}

impl LogLine {
    fn new(expr: &IRBexpr<IRAexpr>, ident: usize) -> Self {
        if matches!(expr, IRBexpr::True | IRBexpr::False | IRBexpr::Cmp(_, _, _)) {
            Self {
                before: Some(format!("{expr:?}")),
                ident,
            }
        } else {
            log::debug!("[constant_fold] {:ident$} {expr:?} {{", "", ident = ident);
            Self {
                before: None,
                ident,
            }
        }
    }

    fn log(self, expr: &mut IRBexpr<IRAexpr>) {
        match self.before {
            Some(before) => {
                log::debug!(
                    "[constant_fold] {:ident$} {} -> {expr:?}",
                    "",
                    before,
                    ident = self.ident
                );
            }
            None => {
                log::debug!(
                    "[constant_fold] {:ident$} }} -> {expr:?}",
                    "",
                    ident = self.ident
                );
            }
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
    fn constant_fold_impl(&mut self, prime: Felt, ident: usize) {
        let log = LogLine::new(self, ident);
        match self {
            IRBexpr::True => {
                log.log(self);
            }
            IRBexpr::False => {
                log.log(self);
            }
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
                log.log(self);
            }
            IRBexpr::And(exprs) => {
                for expr in &mut *exprs {
                    expr.constant_fold_impl(prime, ident + 2);
                }
                // If any value is a literal 'false' convert into IRBexpr::False
                if exprs.iter().any(|expr| {
                    expr.const_value()
                        // If the expr is false-y flip the boolean to return 'true'.
                        .map(|b| !b)
                        // Default to 'false' for non-literal expressions.
                        .unwrap_or_default()
                }) {
                    *self = IRBexpr::False;
                    log.log(self);
                    return;
                }
                // Remove any literal 'true' values.
                exprs.retain(|expr| {
                    expr.const_value()
                        // If the expr is IRBexpr::True we don't want to retain.
                        .map(|b| !b)
                        // Default to true to keep the non-literal values.
                        .unwrap_or(true)
                });
                if exprs.is_empty() {
                    *self = IRBexpr::True;
                }
                log.log(self);
            }
            IRBexpr::Or(exprs) => {
                for expr in &mut *exprs {
                    expr.constant_fold_impl(prime, ident + 2);
                }
                // If any value is a literal 'true' convert into IRBexpr::True.
                if exprs
                    .iter()
                    .any(|expr| expr.const_value().unwrap_or_default())
                {
                    *self = IRBexpr::True;
                    log.log(self);
                    return;
                }
                // Remove any literal 'false' values.
                exprs.retain(|expr| {
                    expr.const_value()
                        // Default to true to keep the non-literal values.
                        .unwrap_or(true)
                });
                if exprs.is_empty() {
                    *self = IRBexpr::False;
                }
                log.log(self);
            }
            IRBexpr::Not(expr) => {
                expr.constant_fold_impl(prime, ident + 2);
                if let Some(b) = expr.const_value() {
                    *self = b.into();
                }
                log.log(self);
            }
            IRBexpr::Det(expr) => expr.constant_fold(prime),
            IRBexpr::Implies(lhs, rhs) => {
                lhs.constant_fold_impl(prime, ident + 2);
                rhs.constant_fold_impl(prime, ident + 2);
                if let Some((lhs, rhs)) = lhs.const_value().zip(rhs.const_value()) {
                    *self = (!lhs || rhs).into();
                }
            }
            IRBexpr::Iff(lhs, rhs) => {
                lhs.constant_fold_impl(prime, ident + 2);
                rhs.constant_fold_impl(prime, ident + 2);
                if let Some((lhs, rhs)) = lhs.const_value().zip(rhs.const_value()) {
                    *self = (lhs == rhs).into();
                }
            }
        }
    }

    pub(crate) fn constant_fold(&mut self, prime: Felt) {
        self.constant_fold_impl(prime, 0)
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
            IRBexpr::Det(_) => {}
            IRBexpr::Implies(lhs, rhs) => {
                lhs.canonicalize();
                rhs.canonicalize();
            }
            IRBexpr::Iff(lhs, rhs) => {
                lhs.canonicalize();
                rhs.canonicalize();
            }
        }
    }
}

impl<T> From<bool> for IRBexpr<T> {
    fn from(value: bool) -> Self {
        if value { IRBexpr::True } else { IRBexpr::False }
    }
}

/// IRBexpr transitively inherits the symbolic equivalence relation.
impl<L, R> EqvRelation<IRBexpr<L>, IRBexpr<R>> for SymbolicEqv
where
    SymbolicEqv: EqvRelation<L, R>,
{
    /// Two boolean expressions are equivalent if they are structurally equal and their inner entities
    /// are equivalent.
    fn equivalent(lhs: &IRBexpr<L>, rhs: &IRBexpr<R>) -> bool {
        match (lhs, rhs) {
            (IRBexpr::Cmp(op1, lhs1, rhs1), IRBexpr::Cmp(op2, lhs2, rhs2)) => {
                op1 == op2 && equiv!(Self | lhs1, lhs2) && equiv!(Self | rhs1, rhs2)
            }
            (IRBexpr::And(lhs), IRBexpr::And(rhs)) => {
                equiv!(Self | lhs, rhs)
            }
            (IRBexpr::Or(lhs), IRBexpr::Or(rhs)) => {
                equiv!(Self | lhs, rhs)
            }
            (IRBexpr::Not(lhs), IRBexpr::Not(rhs)) => {
                equiv!(Self | lhs, rhs)
            }
            (IRBexpr::Det(lhs), IRBexpr::Det(rhs)) => equiv!(Self | lhs, rhs),
            (IRBexpr::Implies(lhs1, rhs1), IRBexpr::Implies(lhs2, rhs2)) => {
                equiv!(Self | lhs1, lhs2) && equiv!(Self | rhs1, rhs2)
            }

            (IRBexpr::Iff(lhs1, rhs1), IRBexpr::Iff(lhs2, rhs2)) => {
                equiv!(Self | lhs1, lhs2) && equiv!(Self | rhs1, rhs2)
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
            IRBexpr::Cmp(cmp_op, lhs, rhs) => write!(f, "({cmp_op} {lhs:?} {rhs:?})",),
            IRBexpr::And(exprs) => write!(f, "(&& {exprs:?})"),
            IRBexpr::Or(exprs) => write!(f, "(|| {exprs:?})"),
            IRBexpr::Not(expr) => write!(f, "(! {expr:?})"),
            IRBexpr::True => write!(f, "(true)"),
            IRBexpr::False => write!(f, "(false)"),
            IRBexpr::Det(expr) => write!(f, "(det {expr:?})"),
            IRBexpr::Implies(lhs, rhs) => write!(f, "(=> {lhs:?} {rhs:?})"),
            IRBexpr::Iff(lhs, rhs) => write!(f, "(<=> {lhs:?} {rhs:?})"),
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
            IRBexpr::Det(expr) => IRBexpr::Det(expr.clone()),
            IRBexpr::Implies(lhs, rhs) => IRBexpr::Implies(lhs.clone(), rhs.clone()),
            IRBexpr::Iff(lhs, rhs) => IRBexpr::Iff(lhs.clone(), rhs.clone()),
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
            (IRBexpr::Det(lhs), IRBexpr::Det(rhs)) => lhs == rhs,
            (IRBexpr::Implies(lhs1, rhs1), IRBexpr::Implies(lhs2, rhs2)) => {
                lhs1 == lhs2 && rhs1 == rhs2
            }
            (IRBexpr::Iff(lhs1, rhs1), IRBexpr::Iff(lhs2, rhs2)) => lhs1 == lhs2 && rhs1 == rhs2,
            _ => false,
        }
    }
}

fn reduce_bool_expr<A, L>(
    exprs: impl IntoIterator<Item = IRBexpr<A>>,
    l: &L,
    cb: impl Fn(&L, &L::CellOutput, &L::CellOutput) -> haloumi_lowering::Result<L::CellOutput>,
) -> haloumi_lowering::Result<L::CellOutput>
where
    A: LowerableExpr,
    L: ExprLowering + ?Sized,
{
    exprs
        .into_iter()
        .map(|e| e.lower(l))
        .reduce(|lhs, rhs| lhs.and_then(|lhs| rhs.and_then(|rhs| cb(l, &lhs, &rhs))))
        .ok_or_else(|| lowering_err!(Error::EmptyBexpr))
        .and_then(identity)
}

impl<F> IRBexpr<F> {}

impl<A: LowerableExpr> LowerableExpr for IRBexpr<A> {
    fn lower<L>(self, l: &L) -> haloumi_lowering::Result<L::CellOutput>
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
            IRBexpr::Det(expr) => expr.lower(l).and_then(|e| l.lower_det(&e)),
            IRBexpr::Implies(lhs, rhs) => {
                let lhs = lhs.lower(l)?;
                let rhs = rhs.lower(l)?;
                l.lower_implies(&lhs, &rhs)
            }
            IRBexpr::Iff(lhs, rhs) => {
                let lhs = lhs.lower(l)?;
                let rhs = rhs.lower(l)?;
                l.lower_iff(&lhs, &rhs)
            }
        }
    }
}
