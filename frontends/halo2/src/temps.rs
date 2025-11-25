//! Types for handling temporaries created during IR generation.

use std::ops::Deref;

use haloumi_ir::{expr::IRAexpr, func::FuncIO};

/// A temporary variable.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Temp(pub(crate) usize);

impl Deref for Temp {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Temp> for FuncIO {
    fn from(value: Temp) -> Self {
        Self::Temp(value.0)
    }
}

/// Wrapper that allows adding temporaries in expressions that don't directly support them
pub enum ExprOrTemp<E> {
    /// Temporary variable.
    Temp(Temp),
    /// Expression.
    Expr(E),
}

impl<E> ExprOrTemp<E> {
    /// Maps the expression.
    pub fn map<O>(self, mut f: impl FnMut(E) -> O) -> ExprOrTemp<O> {
        match self {
            ExprOrTemp::Temp(temp) => ExprOrTemp::Temp(temp),
            ExprOrTemp::Expr(e) => ExprOrTemp::Expr(f(e)),
        }
    }
}

impl<E> From<Temp> for ExprOrTemp<E> {
    fn from(value: Temp) -> Self {
        Self::Temp(value)
    }
}

impl<E: std::fmt::Debug> std::fmt::Debug for ExprOrTemp<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Temp(arg0) => f.debug_tuple("Temp").field(arg0).finish(),
            Self::Expr(arg0) => f.debug_tuple("Expr").field(arg0).finish(),
        }
    }
}

impl<E: Clone> Clone for ExprOrTemp<E> {
    fn clone(&self) -> Self {
        match self {
            Self::Temp(arg0) => Self::Temp(*arg0),
            Self::Expr(arg0) => Self::Expr(arg0.clone()),
        }
    }
}

impl<E: Copy> Copy for ExprOrTemp<E> {}

impl<E> TryFrom<ExprOrTemp<E>> for IRAexpr
where
    IRAexpr: TryFrom<E>,
{
    type Error = <E as TryInto<IRAexpr>>::Error;

    fn try_from(value: ExprOrTemp<E>) -> Result<Self, Self::Error> {
        match value {
            ExprOrTemp::Temp(temp) => Ok(IRAexpr::IO(temp.into())),
            ExprOrTemp::Expr(e) => e.try_into(),
        }
    }
}

/// Generator of temporary variables.
///
/// Handles the generation by implementing [`Iterator`].
#[derive(Debug)]
pub struct Temps {
    count: usize,
}

impl Temps {
    pub(crate) fn new() -> Self {
        Self { count: 0 }
    }
}

impl Iterator for Temps {
    type Item = Temp;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.count;
        self.count += 1;
        Some(Temp(id))
    }
}
