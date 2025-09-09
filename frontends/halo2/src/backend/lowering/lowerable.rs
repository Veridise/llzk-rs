use crate::{backend::func::FuncIO, halo2::Field, ir::stmt::IRStmt};
use anyhow::Result;

use super::{ExprLowering, Lowering};

//pub enum LoweringOutput<L: Lowering + ?Sized> {
//    Value(L::CellOutput),
//    Stmt,
//}
//
//impl<L: Lowering + ?Sized> From<()> for LoweringOutput<L> {
//    fn from(_: ()) -> Self {
//        Self::Stmt
//    }
//}
//
//impl<O: tag::LoweringOutput, L: Lowering<CellOutput = O> + ?Sized> From<O> for LoweringOutput<L> {
//    fn from(value: O) -> Self {
//        Self::Value(value)
//    }
//}

pub trait LowerableExpr {
    type F: Field;

    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering<F = Self::F> + ?Sized;
}

impl<T> LowerableExpr for Result<T>
where
    T: LowerableExpr,
{
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering<F = Self::F> + ?Sized,
    {
        self.and_then(|t| t.lower(l))
    }
}

impl<Lw: LowerableExpr> LowerableExpr for Box<Lw> {
    type F = Lw::F;

    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering<F = Self::F> + ?Sized,
    {
        (*self).lower(l)
    }
}

impl<F: Field> LowerableExpr for (F,) {
    type F = F;

    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering<F = Self::F> + ?Sized,
    {
        l.lower_constant(self.0)
    }
}

pub trait LowerableStmt {
    type F: Field;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized;
}

impl<T> LowerableStmt for Result<T>
where
    T: LowerableStmt,
{
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        self.and_then(|t| t.lower(l))
    }
}

impl<Lw: LowerableStmt> LowerableStmt for Box<Lw> {
    type F = Lw::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        (*self).lower(l)
    }
}

impl<F: Field> LowerableStmt for (F,) {
    type F = F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        Ok(())
    }
}

pub enum LowerableOrIO<L> {
    Lowerable(L),
    IO(FuncIO),
}

impl<L> From<L> for LowerableOrIO<L>
where
    L: LowerableExpr,
{
    fn from(value: L) -> Self {
        Self::Lowerable(value)
    }
}

impl<L> From<FuncIO> for LowerableOrIO<L>
where
    L: LowerableExpr,
{
    fn from(value: FuncIO) -> Self {
        Self::IO(value)
    }
}

impl<LW> LowerableExpr for LowerableOrIO<LW>
where
    LW: LowerableExpr,
{
    type F = LW::F;

    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering<F = Self::F> + ?Sized,
    {
        match self {
            LowerableOrIO::Lowerable(lowerable) => lowerable.lower(l),
            LowerableOrIO::IO(func_io) => l.lower_funcio(func_io),
        }
    }
}

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
    Right: LowerableExpr<F = Left::F>,
{
    type F = Left::F;

    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering<F = Self::F> + ?Sized,
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
    Right: LowerableStmt<F = Left::F>,
{
    type F = Left::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        match self {
            EitherLowerable::Left(left) => left.lower(l),
            EitherLowerable::Right(right) => right.lower(l),
        }
    }
}

impl<T: LowerableStmt, const N: usize> LowerableStmt for [T; N] {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        for e in self {
            e.lower(l)?;
        }
        Ok(())
    }
}

impl<T: LowerableStmt> LowerableStmt for Vec<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        for e in self {
            e.lower(l)?;
        }
        Ok(())
    }
}
