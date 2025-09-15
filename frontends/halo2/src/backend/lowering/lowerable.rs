use crate::{halo2::Field, ir::stmt::IRStmt};
use anyhow::Result;

use super::{ExprLowering, Lowering};

pub trait LowerableExpr {
    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering + ?Sized;
}

impl<T> LowerableExpr for Result<T>
where
    T: LowerableExpr,
{
    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering + ?Sized,
    {
        self.and_then(|t| t.lower(l))
    }
}

impl<Lw: LowerableExpr> LowerableExpr for Box<Lw> {
    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering + ?Sized,
    {
        (*self).lower(l)
    }
}

//impl<F: Field> LowerableExpr for (F,) {
//    type F = F;
//
//    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
//    where
//        L: ExprLowering<F = Self::F> + ?Sized,
//    {
//        l.lower_constant(self.0)
//    }
//}

pub trait LowerableStmt {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized;
}

impl<T> LowerableStmt for Result<T>
where
    T: LowerableStmt,
{
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        self.and_then(|t| t.lower(l))
    }
}

impl<Lw: LowerableStmt> LowerableStmt for Box<Lw> {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        (*self).lower(l)
    }
}

//impl<F: Field> LowerableStmt for (F,) {
//    type F = F;
//
//    fn lower<L>(self, _: &L) -> Result<()>
//    where
//        L: Lowering<F = Self::F> + ?Sized,
//    {
//        Ok(())
//    }
//}

pub enum EitherLowerable<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> EitherLowerable<L, R>
where
    L: Into<R>,
{
    /// If L: Into<R> fold this enum into R
    pub fn fold_right(self) -> R {
        match self {
            EitherLowerable::Left(l) => l.into(),
            EitherLowerable::Right(r) => r,
        }
    }
}

impl<L, R> EitherLowerable<L, R>
where
    R: Into<L>,
{
    /// If R: Into<L> fold this enum into L
    pub fn fold_left(self) -> L {
        match self {
            EitherLowerable::Left(l) => l,
            EitherLowerable::Right(r) => r.into(),
        }
    }
}

impl<T> EitherLowerable<T, T> {
    pub fn unwrap(self) -> T {
        match self {
            EitherLowerable::Left(l) => l,
            EitherLowerable::Right(r) => r,
        }
    }
}

impl<L, R> EitherLowerable<IRStmt<L>, IRStmt<R>>
where
    L: Into<R>,
{
    /// If L: Into<R> fold this enum into R
    pub fn fold_stmt_right(self) -> IRStmt<R> {
        match self {
            EitherLowerable::Left(l) => l.map(&Into::into),
            EitherLowerable::Right(r) => r,
        }
    }
}

impl<L, R> EitherLowerable<IRStmt<L>, IRStmt<R>>
where
    R: Into<L>,
{
    /// If R: Into<L> fold this enum into L
    pub fn fold_stmt_left(self) -> IRStmt<L> {
        match self {
            EitherLowerable::Left(l) => l,
            EitherLowerable::Right(r) => r.map(&Into::into),
        }
    }
}

impl<Left, Right> LowerableExpr for EitherLowerable<Left, Right>
where
    Left: LowerableExpr,
    Right: LowerableExpr,
{
    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering + ?Sized,
    {
        match self {
            EitherLowerable::Left(left) => left.lower(l),
            EitherLowerable::Right(right) => right.lower(l),
        }
    }
}

impl<Left, Right> LowerableStmt for EitherLowerable<Left, Right>
where
    Left: LowerableStmt,
    Right: LowerableStmt,
{
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        match self {
            EitherLowerable::Left(left) => left.lower(l),
            EitherLowerable::Right(right) => right.lower(l),
        }
    }
}

impl<T: LowerableStmt, const N: usize> LowerableStmt for [T; N] {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        for e in self {
            e.lower(l)?;
        }
        Ok(())
    }
}

impl<T: LowerableStmt> LowerableStmt for Vec<T> {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        for e in self {
            e.lower(l)?;
        }
        Ok(())
    }
}
