use super::{equivalency::EqvRelation, expr::IRBexpr, CmpOp};
use crate::backend::{
    func::FuncIO,
    lowering::{
        lowerable::{EitherLowerable, LowerableExpr, LowerableStmt},
        Lowering,
    },
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
    ConstraintCall(Call<T>),
    Constraint(Constraint<T>),
    Comment(Comment<T>),
    AssumeDeterministic(AssumeDeterministic<T>),
    Assert(Assert<T>),
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
    pub fn call(
        callee: impl AsRef<str>,
        inputs: impl IntoIterator<Item = T>,
        outputs: impl IntoIterator<Item = FuncIO>,
    ) -> Self {
        Call::new(callee, inputs, outputs).into()
    }

    pub fn constraint(op: CmpOp, lhs: T, rhs: T) -> Self {
        Constraint::new(op, lhs, rhs).into()
    }

    pub fn comment(s: impl AsRef<str>) -> Self {
        Comment::new(s).into()
    }

    pub fn assume_deterministic(f: impl Into<FuncIO>) -> Self {
        AssumeDeterministic::new(f.into()).into()
    }

    pub fn assert(cond: IRBexpr<T>) -> Self {
        Assert::new(cond).into()
    }

    pub fn seq<I>(stmts: impl IntoIterator<Item = IRStmt<I>>) -> Self
    where
        I: Into<T>,
    {
        Seq::new(stmts).into()
    }

    pub fn empty() -> Self {
        Seq::empty().into()
    }

    pub fn is_empty(&self) -> bool {
        match self {
            IRStmt::Seq(s) => s.is_empty(),
            _ => false,
        }
    }

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

    fn iter<'a>(&'a self) -> IRStmtRefIter<'a, T> {
        IRStmtRefIter { stack: vec![self] }
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
            (IRStmt::Comment(lhs), IRStmt::Comment(rhs)) => {
                <E as EqvRelation<Comment<L>, Comment<R>>>::equivalent(lhs, rhs)
            }
            (IRStmt::AssumeDeterministic(lhs), IRStmt::AssumeDeterministic(rhs)) => {
                <E as EqvRelation<AssumeDeterministic<L>, AssumeDeterministic<R>>>::equivalent(
                    lhs, rhs,
                )
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

impl<T> IRStmt<EitherLowerable<T, T>> {
    pub fn unwrap(self) -> IRStmt<T> {
        self.map(&EitherLowerable::<T, T>::unwrap)
    }
}

macro_rules! impl_from {
    ($inner:ident, $ctor:ident) => {
        impl<T> From<$inner<T>> for IRStmt<T> {
            fn from(value: $inner<T>) -> Self {
                Self::$ctor(value)
            }
        }
    };
    ($name:ident) => {
        impl<T> From<$name<T>> for IRStmt<T> {
            fn from(value: $name<T>) -> Self {
                Self::$name(value)
            }
        }
    };
}

impl_from!(Call, ConstraintCall);
impl_from!(Constraint);
impl_from!(Comment);
impl_from!(AssumeDeterministic);
impl_from!(Assert);
impl_from!(Seq);

impl<T: LowerableExpr> LowerableStmt for IRStmt<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
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
