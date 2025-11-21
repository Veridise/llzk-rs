use crate::{
    error::Error,
    expr::{IRAexpr, IRBexpr},
    stmt::IRStmt,
};
use haloumi_ir_base::{
    equivalency::{EqvRelation, SymbolicEqv},
    felt::Felt,
};
use haloumi_lowering::{
    Lowering,
    lowerable::{LowerableExpr, LowerableStmt},
};

pub struct Assert<T>(IRBexpr<T>);

impl<T> Assert<T> {
    pub fn new(cond: IRBexpr<T>) -> Self {
        Self(cond)
    }

    pub fn cond(&self) -> &IRBexpr<T> {
        &self.0
    }

    pub fn cond_mut(&mut self) -> &mut IRBexpr<T> {
        &mut self.0
    }

    pub fn map<O>(self, f: &impl Fn(T) -> O) -> Assert<O> {
        Assert::new(self.0.map(f))
    }

    pub fn map_into<O>(&self, f: &impl Fn(&T) -> O) -> Assert<O> {
        Assert::new(self.0.map_into(f))
    }

    pub fn try_map<O, E>(self, f: &impl Fn(T) -> Result<O, E>) -> Result<Assert<O>, E> {
        self.0.try_map(f).map(Assert::new)
    }

    pub fn try_map_inplace<E>(&mut self, f: &impl Fn(&mut T) -> Result<(), E>) -> Result<(), E> {
        self.0.try_map_inplace(f)
    }
}

impl Assert<IRAexpr> {
    /// Folds the statements if the expressions are constant.
    /// If a assert-like statement folds into a tautology (i.e. `(= 0 0 )`) gets removed. If it
    /// folds into a unsatisfiable proposition the method returns an error.
    pub fn constant_fold(&mut self, prime: Felt) -> Result<Option<IRStmt<IRAexpr>>, Error> {
        self.0.constant_fold(prime);
        if let Some(b) = self.0.const_value() {
            if b {
                return Ok(Some(IRStmt::empty()));
            } else {
                return Err(Error::FoldedFalseStmt("assert", self.0.clone()));
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
    fn lower<L>(self, l: &L) -> haloumi_lowering::Result<()>
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

impl<L, R> EqvRelation<Assert<L>, Assert<R>> for SymbolicEqv
where
    SymbolicEqv: EqvRelation<L, R>,
{
    fn equivalent(lhs: &Assert<L>, rhs: &Assert<R>) -> bool {
        <SymbolicEqv as EqvRelation<IRBexpr<L>, IRBexpr<R>>>::equivalent(&lhs.0, &rhs.0)
    }
}
