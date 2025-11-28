//! Traits for types that can be lowered.

use crate::error::Error;

use super::{ExprLowering, Lowering, Result};

/// Declares that the type can be lowered into an expression.
pub trait LowerableExpr {
    /// Transforms the value into an expression.
    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering + ?Sized;
}

impl<T, E> LowerableExpr for std::result::Result<T, E>
where
    T: LowerableExpr,
    Error: From<E>,
{
    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering + ?Sized,
    {
        self?.lower(l)
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

/// Declares that the type can be lowered into a statement.
pub trait LowerableStmt {
    /// Emits a statement from the value.
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized;
}

impl<T, E> LowerableStmt for std::result::Result<T, E>
where
    T: LowerableStmt,
    Error: From<E>,
{
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        self?.lower(l)
    }
}

impl<T> LowerableStmt for Option<T>
where
    T: LowerableStmt,
{
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        if let Some(s) = self {
            s.lower(l)?;
        }
        Ok(())
    }
}

impl<T: LowerableStmt> LowerableStmt for Box<T> {
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
