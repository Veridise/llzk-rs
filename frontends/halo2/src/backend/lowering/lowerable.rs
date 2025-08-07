use crate::{backend::func::FuncIO, halo2::Field, ir::stmt::IRStmt};
use anyhow::Result;

use super::{tag, Lowering};

pub enum LoweringOutput<L: Lowering + ?Sized> {
    Value(L::CellOutput),
    Stmt,
}

impl<L: Lowering + ?Sized> From<()> for LoweringOutput<L> {
    fn from(_: ()) -> Self {
        Self::Stmt
    }
}

impl<O: tag::LoweringOutput, L: Lowering<CellOutput = O> + ?Sized> From<O> for LoweringOutput<L> {
    fn from(value: O) -> Self {
        Self::Value(value)
    }
}

pub trait Lowerable {
    type F: Field;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized;
}

impl<T> Lowerable for Result<T>
where
    T: Lowerable,
{
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        self.and_then(|t| t.lower(l))
    }
}

pub enum LowerableOrIO<L> {
    Lowerable(L),
    IO(FuncIO),
}

impl<L> From<L> for LowerableOrIO<L>
where
    L: Lowerable,
{
    fn from(value: L) -> Self {
        Self::Lowerable(value)
    }
}

impl<L> From<FuncIO> for LowerableOrIO<L>
where
    L: Lowerable,
{
    fn from(value: FuncIO) -> Self {
        Self::IO(value)
    }
}

impl<LW> Lowerable for LowerableOrIO<LW>
where
    LW: Lowerable,
{
    type F = LW::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        match self {
            LowerableOrIO::Lowerable(lowerable) => lowerable.lower(l).map(Into::into),
            LowerableOrIO::IO(func_io) => l.lower_funcio(func_io).map(LoweringOutput::Value),
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

impl<Left, Right> Lowerable for EitherLowerable<Left, Right>
where
    Left: Lowerable,
    Right: Lowerable<F = Left::F>,
{
    type F = Left::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        match self {
            EitherLowerable::Left(left) => left.lower(l).map(Into::into),
            EitherLowerable::Right(right) => right.lower(l).map(Into::into),
        }
    }
}

impl<Lw: Lowerable> Lowerable for Box<Lw> {
    type F = Lw::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        (*self).lower(l)
    }
}

impl<F: Field> Lowerable for (F,) {
    type F = F;

    fn lower<L>(self, _: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        Ok(())
    }
}
