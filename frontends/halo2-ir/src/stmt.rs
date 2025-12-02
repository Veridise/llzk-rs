//! Structs for representing statements of the circuit's logic.

use super::expr::IRBexpr;
use crate::{error::Error, expr::IRAexpr, traits::ConstantFolding};
use eqv::{EqvRelation, equiv};
use haloumi_ir_base::{SymbolicEqv, cmp::CmpOp, func::FuncIO};
use haloumi_lowering::{
    Lowering,
    lowerable::{LowerableExpr, LowerableStmt},
};

mod assert;
mod assume_determ;
mod call;
mod comment;
mod constraint;
mod post_cond;
mod seq;

use assert::Assert;
use assume_determ::AssumeDeterministic;
use call::Call;
use comment::Comment;
use constraint::Constraint;
use post_cond::PostCond;
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
    /// A post-condition expression.
    PostCond(PostCond<T>),
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
            (IRStmt::PostCond(lhs), IRStmt::PostCond(rhs)) => lhs.eq(rhs),
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
            IRStmt::PostCond(pc) => write!(f, "{pc:?}"),
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

    /// Creates a post condition formula.
    pub fn post_cond(cond: IRBexpr<T>) -> Self {
        PostCond::new(cond).into()
    }

    /// Creates a constraint between two expressions.
    pub fn constraint(op: CmpOp, lhs: T, rhs: T) -> Self {
        Constraint::new(op, lhs, rhs).into()
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Eq`] between two expressions.
    pub fn eq(lhs: T, rhs: T) -> Self {
        Self::constraint(CmpOp::Eq, lhs, rhs)
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Lt`] between two expressions.
    pub fn lt(lhs: T, rhs: T) -> Self {
        Self::constraint(CmpOp::Lt, lhs, rhs)
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Le`] between two expressions.
    pub fn le(lhs: T, rhs: T) -> Self {
        Self::constraint(CmpOp::Le, lhs, rhs)
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Gt`] between two expressions.
    pub fn gt(lhs: T, rhs: T) -> Self {
        Self::constraint(CmpOp::Gt, lhs, rhs)
    }

    #[inline]
    /// Creates a constraint with [`CmpOp::Ge`] between two expressions.
    pub fn ge(lhs: T, rhs: T) -> Self {
        Self::constraint(CmpOp::Ge, lhs, rhs)
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
            IRStmt::PostCond(pc) => pc.map(f).into(),
            IRStmt::Seq(seq) => Seq::new(seq.into_iter().map(|s| s.map(f))).into(),
        }
    }

    /// Maps the statement's inner type to a tuple with the passed value.
    pub fn with<O>(self, other: O) -> IRStmt<(O, T)>
    where
        O: Clone,
    {
        self.map(&|t| (other.clone(), t))
    }

    /// Maps the statement's inner type to a tuple with the result of the closure.
    pub fn with_fn<O>(self, other: impl Fn() -> O) -> IRStmt<(O, T)> {
        self.map(&|t| (other(), t))
    }

    /// Transforms the inner expression type using [`Into::into`].
    pub fn into<O>(self) -> IRStmt<O>
    where
        O: From<T>,
    {
        self.map(&Into::into)
    }

    /// Transforms the inner expression type using [`From::from`].
    pub fn from<O>(value: IRStmt<O>) -> Self
    where
        O: Into<T>,
    {
        value.map(&Into::into)
    }

    /// Transforms the inner expression type into another, without moving.
    pub fn map_into<O>(&self, f: &impl Fn(&T) -> O) -> IRStmt<O> {
        match self {
            IRStmt::ConstraintCall(call) => call.map_into(f).into(),
            IRStmt::Constraint(constraint) => constraint.map_into(f).into(),
            IRStmt::Comment(comment) => Comment::new(comment.value()).into(),
            IRStmt::AssumeDeterministic(ad) => AssumeDeterministic::new(ad.value()).into(),
            IRStmt::Assert(assert) => assert.map_into(f).into(),
            IRStmt::PostCond(pc) => pc.map_into(f).into(),
            IRStmt::Seq(seq) => Seq::new(seq.iter().map(|s| s.map_into(f))).into(),
        }
    }

    /// Tries to transform the inner expression type into another.
    pub fn try_map<O, E>(self, f: &impl Fn(T) -> Result<O, E>) -> Result<IRStmt<O>, E> {
        Ok(match self {
            IRStmt::ConstraintCall(call) => call.try_map(f)?.into(),
            IRStmt::Constraint(constraint) => constraint.try_map(f)?.into(),
            IRStmt::Comment(comment) => Comment::new(comment.value()).into(),
            IRStmt::AssumeDeterministic(ad) => AssumeDeterministic::new(ad.value()).into(),
            IRStmt::Assert(assert) => assert.try_map(f)?.into(),
            IRStmt::PostCond(pc) => pc.try_map(f)?.into(),
            IRStmt::Seq(seq) => Seq::new(
                seq.into_iter()
                    .map(|s| s.try_map(f))
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .into(),
        })
    }

    /// Tries to modify the inner expression type in place.
    pub fn try_map_inplace<E>(&mut self, f: &impl Fn(&mut T) -> Result<(), E>) -> Result<(), E> {
        match self {
            IRStmt::ConstraintCall(call) => call.try_map_inplace(f),
            IRStmt::Constraint(constraint) => constraint.try_map_inplace(f),
            IRStmt::Comment(_) => Ok(()),
            IRStmt::AssumeDeterministic(_) => Ok(()),
            IRStmt::Assert(assert) => assert.try_map_inplace(f),
            IRStmt::PostCond(pc) => pc.try_map_inplace(f),
            IRStmt::Seq(seq) => {
                for stmt in seq.iter_mut() {
                    stmt.try_map_inplace(f)?;
                }
                Ok(())
            }
        }
    }

    /// Returns an iterator of references to the statements.
    pub fn iter<'a>(&'a self) -> IRStmtRefIter<'a, T> {
        IRStmtRefIter { stack: vec![self] }
    }

    /// Returns an iterator of mutable references to the statements.
    pub fn iter_mut<'a>(&'a mut self) -> IRStmtRefMutIter<'a, T> {
        IRStmtRefMutIter { stack: vec![self] }
    }
}

impl<T> ConstantFolding for IRStmt<T>
where
    T: ConstantFolding + std::fmt::Debug + Clone,
    Error: From<T::Error>,
    T::T: Eq + Ord,
{
    type F = T::F;
    type Error = Error;
    type T = ();

    /// Folds the statements if the expressions are constant.
    /// If a assert-like statement folds into a tautology (i.e. `(= 0 0 )`) gets removed. If it
    /// folds into a unsatisfiable proposition the method returns an error.
    fn constant_fold(&mut self, prime: T::F) -> Result<(), Error> {
        match self {
            IRStmt::ConstraintCall(call) => call.constant_fold(prime)?,
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
            IRStmt::PostCond(pc) => {
                if let Some(replacement) = pc.constant_fold(prime)? {
                    *self = replacement;
                }
            }
            IRStmt::Seq(seq) => seq.constant_fold(prime)?,
        }
        Ok(())
    }
}

impl IRStmt<IRAexpr> {
    /// Matches the statements against a series of known patterns and applies rewrites if able to.
    pub fn canonicalize(&mut self) {
        match self {
            IRStmt::ConstraintCall(_) => {}
            IRStmt::Constraint(constraint) => constraint.canonicalize(),
            IRStmt::Comment(_) => {}
            IRStmt::AssumeDeterministic(_) => {}
            IRStmt::Assert(assert) => assert.canonicalize(),
            IRStmt::PostCond(pc) => pc.canonicalize(),
            IRStmt::Seq(seq) => seq.canonicalize(),
        }
    }
}

/// IRStmt transilitively inherits the [`SymbolicEqv`] equivalence relation.
impl<L, R> EqvRelation<IRStmt<L>, IRStmt<R>> for SymbolicEqv
where
    SymbolicEqv: EqvRelation<L, R> + EqvRelation<FuncIO, FuncIO>,
{
    /// Two statements are equivalent if they are structurally equal and their inner entities
    /// are equivalent.
    fn equivalent(lhs: &IRStmt<L>, rhs: &IRStmt<R>) -> bool {
        std::iter::zip(lhs.iter(), rhs.iter()).all(|(lhs, rhs)| match (lhs, rhs) {
            (IRStmt::ConstraintCall(lhs), IRStmt::ConstraintCall(rhs)) => {
                equiv! { SymbolicEqv | lhs, rhs }
                //<SymbolicEqv as EqvRelation<Call<L>, Call<R>>>::equivalent(lhs, rhs)
            }
            (IRStmt::Constraint(lhs), IRStmt::Constraint(rhs)) => {
                equiv! { SymbolicEqv | lhs, rhs }
                //<SymbolicEqv as EqvRelation<Constraint<L>, Constraint<R>>>::equivalent(lhs, rhs)
            }
            (IRStmt::Comment(_), IRStmt::Comment(_)) => true,
            (IRStmt::AssumeDeterministic(lhs), IRStmt::AssumeDeterministic(rhs)) => {
                equiv! { SymbolicEqv | lhs, rhs }
                //<SymbolicEqv as EqvRelation<AssumeDeterministic, AssumeDeterministic>>::equivalent(
                //    lhs, rhs,
                //)
            }
            (IRStmt::Assert(lhs), IRStmt::Assert(rhs)) => {
                equiv! { SymbolicEqv | lhs, rhs }
                //<SymbolicEqv as EqvRelation<Assert<L>, Assert<R>>>::equivalent(lhs, rhs)
            }
            (IRStmt::PostCond(lhs), IRStmt::PostCond(rhs)) => {
                equiv! { SymbolicEqv | lhs, rhs }
                //<SymbolicEqv as EqvRelation<PostCond<L>, PostCond<R>>>::equivalent(lhs, rhs)
            }
            (IRStmt::Seq(_), _) | (_, IRStmt::Seq(_)) => unreachable!(),
            _ => false,
        })
    }
}

/// Iterator over references.
#[derive(Debug)]
pub struct IRStmtRefIter<'a, T> {
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

/// Iterator over mutable references.
#[derive(Debug)]
pub struct IRStmtRefMutIter<'a, T> {
    stack: Vec<&'a mut IRStmt<T>>,
}

impl<'a, T> Iterator for IRStmtRefMutIter<'a, T> {
    type Item = &'a mut IRStmt<T>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node {
                IRStmt::Seq(children) => {
                    // Reverse to preserve left-to-right order
                    self.stack.extend(children.iter_mut().rev());
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

impl<'a, T> IntoIterator for &'a IRStmt<T> {
    type Item = Self;

    type IntoIter = IRStmtRefIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut IRStmt<T> {
    type Item = Self;

    type IntoIter = IRStmtRefMutIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
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
impl<T> From<PostCond<T>> for IRStmt<T> {
    fn from(value: PostCond<T>) -> Self {
        Self::PostCond(value)
    }
}
impl<T> From<Seq<T>> for IRStmt<T> {
    fn from(value: Seq<T>) -> Self {
        Self::Seq(value)
    }
}

impl<T: LowerableExpr> LowerableStmt for IRStmt<T> {
    fn lower<L>(self, l: &L) -> haloumi_lowering::Result<()>
    where
        L: Lowering + ?Sized,
    {
        match self {
            Self::ConstraintCall(call) => call.lower(l),
            Self::Constraint(constraint) => constraint.lower(l),
            Self::Comment(comment) => comment.lower(l),
            Self::AssumeDeterministic(ad) => ad.lower(l),
            Self::Assert(assert) => assert.lower(l),
            Self::PostCond(pc) => pc.lower(l),
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
            Self::PostCond(e) => e.clone().into(),
            Self::Seq(stmts) => stmts.clone().into(),
        }
    }
}

#[cfg(test)]
mod test;
