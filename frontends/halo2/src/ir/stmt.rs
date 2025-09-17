//! Structs for representing statements of the circuit's logic.

use super::{equivalency::EqvRelation, expr::IRBexpr, CmpOp};
use crate::{
    backend::{
        func::FuncIO,
        lowering::{
            lowerable::{LowerableExpr, LowerableStmt},
            Lowering,
        },
    },
    ir::expr::{Felt, IRAexpr},
};
use anyhow::Result;

mod assert;
mod assume_determ;
mod call;
mod comment;
mod constraint;
mod seq;

use assert::Assert;
use assume_determ::AssumeDeterministic;
use call::Call;
use comment::Comment;
use constraint::Constraint;
use seq::Seq;

/// IR for operations that occur in the main circuit.
pub enum IRStmt<T> {
    /// A call to another module.
    ConstraintCall(Call<T>),
    /// A constraint between two expressions.
    Constraint(Constraint<T>),
    /// A text comment.
    Comment(Comment),
    /// Indicates that a [`FuncIO`] must be assumed deterministic by the backend.
    AssumeDeterministic(AssumeDeterministic),
    /// A constraint that a [`IRBexpr`] must be true.
    Assert(Assert<T>),
    /// A sequence of statements.
    Seq(Seq<T>),
}

impl<T: PartialEq> PartialEq for IRStmt<T> {
    /// Equality is defined by the sequence of statements regardless of how they are structured
    /// inside.
    ///
    /// For example:
    ///     Seq([a, Seq([b, c])]) == Seq([a, b, c])
    ///     a == Seq([a])
    fn eq(&self, other: &Self) -> bool {
        std::iter::zip(self.iter(), other.iter()).all(|(lhs, rhs)| match (lhs, rhs) {
            (IRStmt::ConstraintCall(lhs), IRStmt::ConstraintCall(rhs)) => lhs.eq(rhs),
            (IRStmt::Constraint(lhs), IRStmt::Constraint(rhs)) => lhs.eq(rhs),
            (IRStmt::Comment(lhs), IRStmt::Comment(rhs)) => lhs.eq(rhs),
            (IRStmt::AssumeDeterministic(lhs), IRStmt::AssumeDeterministic(rhs)) => lhs.eq(rhs),
            (IRStmt::Assert(lhs), IRStmt::Assert(rhs)) => lhs.eq(rhs),
            (IRStmt::Seq(_), _) | (_, IRStmt::Seq(_)) => unreachable!(),
            _ => false,
        })
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for IRStmt<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRStmt::ConstraintCall(call) => write!(f, "{call:?}"),
            IRStmt::Constraint(constraint) => write!(f, "{constraint:?}"),
            IRStmt::Comment(comment) => write!(f, "{comment:?}"),
            IRStmt::AssumeDeterministic(assume_deterministic) => {
                write!(f, "{assume_deterministic:?}")
            }
            IRStmt::Assert(assert) => write!(f, "{assert:?}"),
            IRStmt::Seq(seq) => write!(f, "{seq:?}"),
        }
    }
}

impl<T> IRStmt<T> {
    /// Creates a call to another module.
    pub fn call(
        callee: impl AsRef<str>,
        inputs: impl IntoIterator<Item = T>,
        outputs: impl IntoIterator<Item = FuncIO>,
    ) -> Self {
        Call::new(callee, inputs, outputs).into()
    }

    /// Creates a constraint between two expressions.
    pub fn constraint(op: CmpOp, lhs: T, rhs: T) -> Self {
        Constraint::new(op, lhs, rhs).into()
    }

    /// Creates a text comment.
    pub fn comment(s: impl AsRef<str>) -> Self {
        Comment::new(s).into()
    }

    /// Indicates that the [`FuncIO`] must be assumed deterministic by the backend.
    pub fn assume_deterministic(f: impl Into<FuncIO>) -> Self {
        AssumeDeterministic::new(f.into()).into()
    }

    /// Creates an assertion in the circuit.
    pub fn assert(cond: IRBexpr<T>) -> Self {
        Assert::new(cond).into()
    }

    /// Creates a statement that is a sequence of other statements.
    pub fn seq<I>(stmts: impl IntoIterator<Item = IRStmt<I>>) -> Self
    where
        I: Into<T>,
    {
        Seq::new(stmts).into()
    }

    /// Creates an empty statement.
    pub fn empty() -> Self {
        Seq::empty().into()
    }

    /// Returns true if the statement is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            IRStmt::Seq(s) => s.is_empty(),
            _ => false,
        }
    }

    /// Transforms the inner expression type into another.
    pub fn map<O>(self, f: &impl Fn(T) -> O) -> IRStmt<O> {
        match self {
            IRStmt::ConstraintCall(call) => call.map(f).into(),
            IRStmt::Constraint(constraint) => constraint.map(f).into(),
            IRStmt::Comment(comment) => Comment::new(comment.value()).into(),
            IRStmt::AssumeDeterministic(ad) => AssumeDeterministic::new(ad.value()).into(),
            IRStmt::Assert(assert) => assert.map(f).into(),
            IRStmt::Seq(seq) => Seq::new(seq.into_iter().map(|s| s.map(f))).into(),
        }
    }

    /// Transforms the inner expression type into another, without moving.
    pub fn map_into<O>(&self, f: &impl Fn(&T) -> O) -> IRStmt<O> {
        match self {
            IRStmt::ConstraintCall(call) => call.map_into(f).into(),
            IRStmt::Constraint(constraint) => constraint.map_into(f).into(),
            IRStmt::Comment(comment) => Comment::new(comment.value()).into(),
            IRStmt::AssumeDeterministic(ad) => AssumeDeterministic::new(ad.value()).into(),
            IRStmt::Assert(assert) => assert.map_into(f).into(),
            IRStmt::Seq(seq) => Seq::new(seq.iter().map(|s| s.map_into(f))).into(),
        }
    }

    /// Tries to transform the inner expression type into another.
    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<IRStmt<O>> {
        Ok(match self {
            IRStmt::ConstraintCall(call) => call.try_map(f)?.into(),
            IRStmt::Constraint(constraint) => constraint.try_map(f)?.into(),
            IRStmt::Comment(comment) => Comment::new(comment.value()).into(),
            IRStmt::AssumeDeterministic(ad) => AssumeDeterministic::new(ad.value()).into(),
            IRStmt::Assert(assert) => assert.try_map(f)?.into(),
            IRStmt::Seq(seq) => Seq::new(
                seq.into_iter()
                    .map(|s| s.try_map(f))
                    .collect::<Result<Vec<_>>>()?,
            )
            .into(),
        })
    }

    /// Tries to modify the inner expression type in place.
    pub fn try_map_inplace(&mut self, f: &impl Fn(&mut T) -> Result<()>) -> Result<()> {
        match self {
            IRStmt::ConstraintCall(call) => call.try_map_inplace(f),
            IRStmt::Constraint(constraint) => constraint.try_map_inplace(f),
            IRStmt::Comment(_) => Ok(()),
            IRStmt::AssumeDeterministic(_) => Ok(()),
            IRStmt::Assert(assert) => assert.try_map_inplace(f),
            IRStmt::Seq(seq) => {
                for stmt in seq.iter_mut() {
                    stmt.try_map_inplace(f)?;
                }
                Ok(())
            }
        }
    }

    fn iter<'a>(&'a self) -> IRStmtRefIter<'a, T> {
        IRStmtRefIter { stack: vec![self] }
    }
}

impl IRStmt<IRAexpr> {
    /// Folds the statements if the expressions are constant.
    /// If a assert-like statement folds into a tautology (i.e. `(= 0 0 )`) gets removed. If it
    /// folds into a unsatisfiable proposition the method returns an error.
    pub(crate) fn constant_fold(&mut self, prime: Felt) -> Result<()> {
        match self {
            IRStmt::ConstraintCall(call) => call.constant_fold(prime),
            IRStmt::Constraint(constraint) => {
                if let Some(replacement) = constraint.constant_fold(prime)? {
                    *self = replacement;
                }
            }
            IRStmt::Comment(_) => {}
            IRStmt::AssumeDeterministic(_) => {}
            IRStmt::Assert(assert) => {
                if let Some(replacement) = assert.constant_fold(prime)? {
                    *self = replacement;
                }
            }
            IRStmt::Seq(seq) => seq.constant_fold(prime)?,
        }
        Ok(())
    }

    /// Matches the statements against a series of known patterns and applies rewrites if able to.
    pub(crate) fn canonicalize(&mut self) {
        match self {
            IRStmt::ConstraintCall(_) => {}
            IRStmt::Constraint(constraint) => constraint.canonicalize(),
            IRStmt::Comment(_) => {}
            IRStmt::AssumeDeterministic(_) => {}
            IRStmt::Assert(assert) => assert.canonicalize(),
            IRStmt::Seq(seq) => seq.canonicalize(),
        }
    }
}

/// IRStmt transilitively inherits any equivalence relation.
impl<L, R, E> EqvRelation<IRStmt<L>, IRStmt<R>> for E
where
    E: EqvRelation<L, R> + EqvRelation<FuncIO, FuncIO>,
{
    /// Two statements are equivalent if they are structurally equal and their inner entities
    /// are equivalent.
    fn equivalent(lhs: &IRStmt<L>, rhs: &IRStmt<R>) -> bool {
        std::iter::zip(lhs.iter(), rhs.iter()).all(|(lhs, rhs)| match (lhs, rhs) {
            (IRStmt::ConstraintCall(lhs), IRStmt::ConstraintCall(rhs)) => {
                <E as EqvRelation<Call<L>, Call<R>>>::equivalent(lhs, rhs)
            }
            (IRStmt::Constraint(lhs), IRStmt::Constraint(rhs)) => {
                <E as EqvRelation<Constraint<L>, Constraint<R>>>::equivalent(lhs, rhs)
            }
            (IRStmt::Comment(_), IRStmt::Comment(_)) => true,
            (IRStmt::AssumeDeterministic(lhs), IRStmt::AssumeDeterministic(rhs)) => {
                <E as EqvRelation<AssumeDeterministic, AssumeDeterministic>>::equivalent(lhs, rhs)
            }
            (IRStmt::Assert(lhs), IRStmt::Assert(rhs)) => {
                <E as EqvRelation<Assert<L>, Assert<R>>>::equivalent(lhs, rhs)
            }
            (IRStmt::Seq(_), _) | (_, IRStmt::Seq(_)) => unreachable!(),
            _ => false,
        })
    }
}

struct IRStmtRefIter<'a, T> {
    stack: Vec<&'a IRStmt<T>>,
}

impl<'a, T> Iterator for IRStmtRefIter<'a, T> {
    type Item = &'a IRStmt<T>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node {
                IRStmt::Seq(children) => {
                    // Reverse to preserve left-to-right order
                    self.stack.extend(children.iter().rev());
                }
                stmt => return Some(stmt),
            }
        }
        None
    }
}

impl<T> Default for IRStmt<T> {
    fn default() -> Self {
        Self::empty()
    }
}

/// Iterator of statements.
#[derive(Debug)]
pub struct IRStmtIter<T> {
    stack: Vec<IRStmt<T>>,
}

impl<T> Iterator for IRStmtIter<T> {
    type Item = IRStmt<T>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node {
                IRStmt::Seq(children) => {
                    // Reverse to preserve left-to-right order
                    self.stack.extend(children.into_iter().rev());
                }
                stmt => return Some(stmt),
            }
        }
        None
    }
}

impl<T> IntoIterator for IRStmt<T> {
    type Item = Self;

    type IntoIter = IRStmtIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IRStmtIter { stack: vec![self] }
    }
}

impl<I> FromIterator<IRStmt<I>> for IRStmt<I> {
    fn from_iter<T: IntoIterator<Item = IRStmt<I>>>(iter: T) -> Self {
        Self::seq(iter)
    }
}

impl<T> From<Call<T>> for IRStmt<T> {
    fn from(value: Call<T>) -> Self {
        Self::ConstraintCall(value)
    }
}
impl<T> From<Constraint<T>> for IRStmt<T> {
    fn from(value: Constraint<T>) -> Self {
        Self::Constraint(value)
    }
}
impl<T> From<Comment> for IRStmt<T> {
    fn from(value: Comment) -> Self {
        Self::Comment(value)
    }
}
impl<T> From<AssumeDeterministic> for IRStmt<T> {
    fn from(value: AssumeDeterministic) -> Self {
        Self::AssumeDeterministic(value)
    }
}
impl<T> From<Assert<T>> for IRStmt<T> {
    fn from(value: Assert<T>) -> Self {
        Self::Assert(value)
    }
}
impl<T> From<Seq<T>> for IRStmt<T> {
    fn from(value: Seq<T>) -> Self {
        Self::Seq(value)
    }
}

impl<T: LowerableExpr> LowerableStmt for IRStmt<T> {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        match self {
            Self::ConstraintCall(call) => call.lower(l),
            Self::Constraint(constraint) => constraint.lower(l),
            Self::Comment(comment) => comment.lower(l),
            Self::AssumeDeterministic(ad) => ad.lower(l),
            Self::Assert(assert) => assert.lower(l),
            Self::Seq(seq) => seq.lower(l),
        }
    }
}

impl<T: Clone> Clone for IRStmt<T> {
    fn clone(&self) -> Self {
        match self {
            Self::ConstraintCall(call) => call.clone().into(),
            Self::Constraint(c) => c.clone().into(),
            Self::Comment(c) => c.clone().into(),
            Self::AssumeDeterministic(func_io) => func_io.clone().into(),
            Self::Assert(e) => e.clone().into(),
            Self::Seq(stmts) => stmts.clone().into(),
        }
    }
}

#[cfg(test)]
mod test;
