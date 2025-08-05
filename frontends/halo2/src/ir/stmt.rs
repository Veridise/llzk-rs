use std::{convert::identity, marker::PhantomData};

use anyhow::Result;

use crate::backend::{
    func::FuncIO,
    lowering::{EitherLowerable, Lowerable, Lowering, LoweringOutput},
};

mod assert;
mod assume_determ;
mod call;
mod comment;
mod constraint;
mod seq;

pub use constraint::CmpOp;

use assert::Assert;
use assume_determ::AssumeDeterministic;
use call::Call;
use comment::Comment;
use constraint::Constraint;
use seq::Seq;

// Temporary alias
pub type IRStmt<T> = CircuitStmt<T>;

/// IR for operations that occur in the main circuit.
pub enum CircuitStmt<T> {
    ConstraintCall(Call<T>),
    Constraint(Constraint<T>),
    Comment(Comment<T>),
    AssumeDeterministic(AssumeDeterministic<T>),
    Assert(Assert<T>),
    Seq(Seq<T>),
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

    pub fn assert(cond: T) -> Self {
        Assert::new(cond).into()
    }

    pub fn seq<I>(stmts: impl IntoIterator<Item = IRStmt<I>>) -> Self
    where
        I: Into<T>,
    {
        Seq::new(stmts).into()
    }

    pub fn map<O>(self, f: &impl Fn(T) -> O) -> IRStmt<O> {
        match self {
            CircuitStmt::ConstraintCall(call) => call.map(f).into(),
            CircuitStmt::Constraint(constraint) => constraint.map(f).into(),
            CircuitStmt::Comment(comment) => Comment::new(comment.value()).into(),
            CircuitStmt::AssumeDeterministic(ad) => AssumeDeterministic::new(ad.value()).into(),
            CircuitStmt::Assert(assert) => assert.map(f).into(),
            CircuitStmt::Seq(seq) => Seq::new(seq.into_iter().map(|s| s.map(f))).into(),
        }
    }

    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<IRStmt<O>> {
        Ok(match self {
            CircuitStmt::ConstraintCall(call) => call.try_map(f)?.into(),
            CircuitStmt::Constraint(constraint) => constraint.try_map(f)?.into(),
            CircuitStmt::Comment(comment) => Comment::new(comment.value()).into(),
            CircuitStmt::AssumeDeterministic(ad) => AssumeDeterministic::new(ad.value()).into(),
            CircuitStmt::Assert(assert) => assert.try_map(f)?.into(),
            CircuitStmt::Seq(seq) => Seq::new(
                seq.into_iter()
                    .map(|s| s.try_map(f))
                    .collect::<Result<Vec<_>>>()?,
            )
            .into(),
        })
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

macro_rules! chain_lowerable_stmts {
    ($head:expr) => {
        $head.into_iter()
    };
    ($head:expr, $($tail:expr),* $(,)?) => {
{
        $head.into_iter().map(|stmt| stmt.map(&crate::backend::lowering::EitherLowerable::Left)).chain(chain_lowerable_stmts!($( $tail ),*).map(|stmt| stmt.map(&crate::backend::lowering::EitherLowerable::Right)))

        }
    };
}
pub(crate) use chain_lowerable_stmts;

//impl<I, O> From<IRStmt<I>> for IRStmt<O>
//where
//    I: Into<O>,
//{
//    fn from(value: IRStmt<I>) -> Self {
//        value.map(&Into::into)
//    }
//}

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

impl<T: Lowerable> Lowerable for IRStmt<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        match self {
            Self::ConstraintCall(call) => call.lower(l).map(Into::into),
            Self::Constraint(constraint) => constraint.lower(l).map(Into::into),
            Self::Comment(comment) => comment.lower(l).map(Into::into),
            Self::AssumeDeterministic(ad) => ad.lower(l).map(Into::into),
            Self::Assert(assert) => assert.lower(l).map(Into::into),
            Self::Seq(seq) => seq.lower(l).map(Into::into),
        }
    }
}

impl<T: Clone> Clone for CircuitStmt<T> {
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
mod test {
    use super::*;

    type S = IRStmt<()>;

    #[test]
    fn iterator_nested_seqs() {
        let nested = S::seq([S::assert(()), S::seq([S::assert(()), S::assert(())])]);
        let expected = vec![S::assert(()); 3];
        let output = nested.into_iter().collect::<Vec<_>>();
        assert_eq!(expected, output);
    }
}
