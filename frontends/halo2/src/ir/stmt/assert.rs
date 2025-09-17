use anyhow::Result;

use crate::{
    backend::lowering::{
        lowerable::{LowerableExpr, LowerableStmt},
        Lowering,
    },
    ir::{
        equivalency::EqvRelation,
        expr::{Felt, IRAexpr, IRBexpr},
        stmt::IRStmt,
    },
};

pub struct Assert<T>(IRBexpr<T>);

impl<T> Assert<T> {
    pub fn new(cond: IRBexpr<T>) -> Self {
        Self(cond)
    }

    pub fn map<O>(self, f: &impl Fn(T) -> O) -> Assert<O> {
        Assert::new(self.0.map(f))
    }

    pub fn map_into<O>(&self, f: &impl Fn(&T) -> O) -> Assert<O> {
        Assert::new(self.0.map_into(f))
    }

    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<Assert<O>> {
        self.0.try_map(f).map(Assert::new)
    }

    pub fn try_map_inplace(&mut self, f: &impl Fn(&mut T) -> Result<()>) -> Result<()> {
        self.0.try_map_inplace(f)
    }
}

impl Assert<IRAexpr> {
    /// Folds the statements if the expressions are constant.
    /// If a assert-like statement folds into a tautology (i.e. `(= 0 0 )`) gets removed. If it
    /// folds into a unsatisfiable proposition the method returns an error.
    pub fn constant_fold(&mut self, prime: Felt) -> Result<Option<IRStmt<IRAexpr>>> {
        self.0.constant_fold(prime);
        if let Some(b) = self.0.const_value() {
            if b {
                return Ok(Some(IRStmt::empty()));
            } else {
                return Err(anyhow::anyhow!(
                    "Detected assert statement with predicate evaluating to 'false'"
                ));
            }
        }
        Ok(None)
    }

    /// Matches the statements against a series of known patterns and applies rewrites if able to.
    pub(crate) fn canonicalize(&mut self) {
        self.0.canonicalize();
    }
}

impl<T: LowerableExpr> LowerableStmt for Assert<T> {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        l.generate_assert(&self.0.lower(l)?)
    }
}

impl<T: Clone> Clone for Assert<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: PartialEq> PartialEq for Assert<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Assert<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "assert ")?;
        if f.alternate() {
            write!(f, "{:#?}", self.0)
        } else {
            write!(f, "{:?}", self.0)
        }
    }
}

impl<L, R, E> EqvRelation<Assert<L>, Assert<R>> for E
where
    E: EqvRelation<L, R>,
{
    fn equivalent(lhs: &Assert<L>, rhs: &Assert<R>) -> bool {
        <E as EqvRelation<IRBexpr<L>, IRBexpr<R>>>::equivalent(&lhs.0, &rhs.0)
    }
}
