use anyhow::Result;

use crate::backend::lowering::{
    lowerable::{LowerableExpr, LowerableStmt},
    Lowering,
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
}

impl<T: LowerableExpr> LowerableStmt for Seq<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
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
