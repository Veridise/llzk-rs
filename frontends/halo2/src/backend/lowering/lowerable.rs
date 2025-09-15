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
