use anyhow::Result;

use crate::{
    backend::lowering::{
        lowerable::{LowerableExpr, LowerableStmt},
        Lowering,
    },
    ir::expr::{Felt, IRAexpr},
};

use super::IRStmt;

pub struct Seq<T>(Vec<IRStmt<T>>);

impl<T> Seq<T> {
    pub fn new<I: Into<T>>(stmts: impl IntoIterator<Item = IRStmt<I>>) -> Self {
        Self(
            stmts
                .into_iter()
                .map(|stmt| stmt.map(&Into::into))
                .collect(),
        )
    }

    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter<'a>(&'a self) -> std::slice::Iter<'a, IRStmt<T>> {
        self.0.iter()
    }

    pub fn iter_mut<'a>(&'a mut self) -> std::slice::IterMut<'a, IRStmt<T>> {
        self.0.iter_mut()
    }
}

impl Seq<IRAexpr> {
    /// Folds the statements if the expressions are constant.
    /// If a assert-like statement folds into a tautology (i.e. `(= 0 0 )`) gets removed. If it
    /// folds into a unsatisfiable proposition the method returns an error.
    pub fn constant_fold(&mut self, prime: Felt) -> Result<()> {
        self.0
            .iter_mut()
            .try_for_each(|inner| inner.constant_fold(prime))
    }

    /// Matches the statements against a series of known patterns and applies rewrites if able to.
    pub(crate) fn canonicalize(&mut self) {
        for inner in &mut self.0 {
            inner.canonicalize();
        }
    }
}

impl<T: LowerableExpr> LowerableStmt for Seq<T> {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        self.0.into_iter().try_for_each(|s| s.lower(l))
    }
}

impl<T: Clone> Clone for Seq<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> IntoIterator for Seq<T> {
    type Item = IRStmt<T>;

    type IntoIter = <Vec<IRStmt<T>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T: PartialEq> PartialEq for Seq<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Seq<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            writeln!(f, "{{")?;
            self.0
                .iter()
                .enumerate()
                .try_for_each(|(idx, stmt)| writeln!(f, "{idx}: {stmt:#?};"))?;
            writeln!(f, "}}")
        } else {
            write!(f, "{{ ")?;
            self.0
                .iter()
                .try_for_each(|stmt| write!(f, "{:?}; ", stmt))?;
            write!(f, " }}")
        }
    }
}
