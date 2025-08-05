use anyhow::Result;

use crate::backend::lowering::{Lowerable, Lowering, LoweringOutput};

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
}

impl<T: Lowerable> Lowerable for Seq<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        self.0.into_iter().try_for_each(|s| l.lower_stmt(s))
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
