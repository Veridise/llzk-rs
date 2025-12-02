//! Structs for handling boolean expressions.

use crate::error::Error;
use crate::traits::ConstantFolding;
use crate::{canon::canonicalize_constraint, expr::IRAexpr};
use eqv::{EqvRelation, equiv};
use haloumi_ir_base::SymbolicEqv;
use haloumi_ir_base::cmp::CmpOp;
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
    /// Returns true if the formula is an atom.
    ///
    /// Atom in this context means that it doesn't have other formulas inside it.
    /// The negation of an atom is considered an atom (due to double-negation elimination).
    pub fn is_atom(&self) -> bool {
        match self {
            IRBexpr::True | IRBexpr::False | IRBexpr::Cmp(_, _, _) | IRBexpr::Det(_) => true,
            IRBexpr::And(_) | IRBexpr::Or(_) | IRBexpr::Iff(_, _) | IRBexpr::Implies(_, _) => false,
            IRBexpr::Not(expr) => expr.is_atom(),
        }
    }

    /// Returns true if the formula is in DNF.
    pub fn is_dnf(&self) -> bool {
        match self {
            IRBexpr::True | IRBexpr::False | IRBexpr::Cmp(_, _, _) | IRBexpr::Det(_) => true,
            IRBexpr::Or(exprs) => exprs.iter().all(Self::is_dnf),
            IRBexpr::And(exprs) => exprs.iter().all(Self::is_atom),
            IRBexpr::Not(expr) => expr.is_atom(),
            _ => false,
        }
    }

    fn apply_dnf_patterns(&mut self)
    where
        T: Clone,
    {
        match self {
            IRBexpr::True => {}
            IRBexpr::False => {}
            IRBexpr::Cmp(_, _, _) => {}
            IRBexpr::And(irbexprs) => todo!(),
            IRBexpr::Or(irbexprs) => todo!(),
            IRBexpr::Not(expr) => {
                expr.apply_dnf_patterns();
                match &mut **expr {
                    IRBexpr::True => {
                        *self = IRBexpr::False;
                    }
                    IRBexpr::False => {
                        *self = IRBexpr::True;
                    }
                    IRBexpr::And(exprs) => {
                        *self = IRBexpr::Or(std::mem::take(exprs).into_iter().map(|e| !e).collect())
                    }
                    IRBexpr::Or(exprs) => {
                        *self =
                            IRBexpr::And(std::mem::take(exprs).into_iter().map(|e| !e).collect())
                    }
                    IRBexpr::Not(expr) => *self = std::mem::replace(expr.as_mut(), IRBexpr::False),
                    IRBexpr::Implies(_, _) => unreachable!(),
                    IRBexpr::Iff(_, _) => unreachable!(),
                    _ => {}
                }
            }
            IRBexpr::Det(_) => {}
            IRBexpr::Implies(lhs, rhs) => {
                lhs.apply_dnf_patterns();
                rhs.apply_dnf_patterns();
                let lhs = IRBexpr::Not(std::mem::replace(lhs, Box::new(IRBexpr::False)));
                let rhs = std::mem::replace(rhs.as_mut(), IRBexpr::False);
                *self = lhs | rhs;
            }
            IRBexpr::Iff(lhs, rhs) => {
                lhs.apply_dnf_patterns();
                rhs.apply_dnf_patterns();
                let new_lhs = IRBexpr::Implies(lhs.clone(), rhs.clone());
                let new_rhs = IRBexpr::Implies(
                    std::mem::replace(rhs, Box::new(IRBexpr::False)),
                    std::mem::replace(lhs, Box::new(IRBexpr::False)),
                );
                *self = new_lhs & new_rhs;
            }
        }
    }

    /// Converts the formula into DNF.
    pub fn into_dnf(&mut self) {}

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
    fn new<T: std::fmt::Debug>(expr: &IRBexpr<T>, ident: usize) -> Self {
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

    fn log<T: std::fmt::Debug>(self, expr: &mut IRBexpr<T>) {
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

impl<T> IRBexpr<T>
where
    T: ConstantFolding + std::fmt::Debug,
    T::T: Eq + Ord,
{
    /// Folds the expression if the values are constant.
    fn constant_fold_impl(&mut self, prime: T::F, indent: usize) -> Result<(), T::Error> {
        let log = LogLine::new(self, indent);
        match self {
            IRBexpr::True => {
                log.log(self);
            }
            IRBexpr::False => {
                log.log(self);
            }
            IRBexpr::Cmp(op, lhs, rhs) => {
                lhs.constant_fold(prime)?;
                rhs.constant_fold(prime)?;
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
                    expr.constant_fold_impl(prime, indent + 2)?;
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
                    return Ok(());
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
                    expr.constant_fold_impl(prime, indent + 2)?;
                }
                // If any value is a literal 'true' convert into IRBexpr::True.
                if exprs
                    .iter()
                    .any(|expr| expr.const_value().unwrap_or_default())
                {
                    *self = IRBexpr::True;
                    log.log(self);
                    return Ok(());
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
                expr.constant_fold_impl(prime, indent + 2)?;
                if let Some(b) = expr.const_value() {
                    *self = b.into();
                }
                log.log(self);
            }
            IRBexpr::Det(expr) => expr.constant_fold(prime)?,
            IRBexpr::Implies(lhs, rhs) => {
                lhs.constant_fold_impl(prime, indent + 2)?;
                rhs.constant_fold_impl(prime, indent + 2)?;
                if let Some((lhs, rhs)) = lhs.const_value().zip(rhs.const_value()) {
                    *self = (!lhs || rhs).into();
                }
            }
            IRBexpr::Iff(lhs, rhs) => {
                lhs.constant_fold_impl(prime, indent + 2)?;
                rhs.constant_fold_impl(prime, indent + 2)?;
                if let Some((lhs, rhs)) = lhs.const_value().zip(rhs.const_value()) {
                    *self = (lhs == rhs).into();
                }
            }
        }
        Ok(())
    }
}

impl<T> ConstantFolding for IRBexpr<T>
where
    T: ConstantFolding + std::fmt::Debug,
    T::T: Eq + Ord,
{
    type F = T::F;
    type T = bool;

    type Error = T::Error;

    fn constant_fold(&mut self, prime: Self::F) -> Result<(), Self::Error> {
        self.constant_fold_impl(prime, 0)
    }

    /// Returns `Some(true)` or `Some(false)` if the expression is constant, `None` otherwise.
    fn const_value(&self) -> Option<bool> {
        match self {
            IRBexpr::True => Some(true),
            IRBexpr::False => Some(false),
            _ => None,
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

#[inline]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_dnf() {
        let a = IRBexpr::Det("A");
        let b = IRBexpr::Det("B");
        let c = IRBexpr::Det("C");
        let d = IRBexpr::Det("D");
        let e = IRBexpr::Det("E");
        let f = IRBexpr::Det("F");
        // A
        assert!(a.clone().is_dnf());
        // (A /\ B)
        assert!((a.clone() & b.clone()).is_dnf());
        // (A /\ B) \/ C
        assert!(((a.clone() & b.clone()) | c.clone()).is_dnf());
        // (A /\ !B /\ !C) \/ (!D /\ E /\ F /\ D /\ F)
        assert!(
            ((a.clone() & !b.clone() & !c.clone()) | (!d.clone() & e & f.clone() & d.clone() & f))
                .is_dnf()
        );

        // Not in DNF
        // !(A \/ B)
        assert!(!(!(a.clone() | b.clone())).is_dnf());
        // !(A /\ B) \/ C
        assert!(!(!(a.clone() & b.clone()) | c.clone()).is_dnf());
        // A \/ (B /\ (C \/ D))
        assert!(!(a | (b & (c | d))).is_dnf());
    }
}
