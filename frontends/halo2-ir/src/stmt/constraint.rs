use crate::{
    canon::canonicalize_constraint,
    error::Error,
    expr::{IRAexpr, IRBexpr},
    stmt::IRStmt,
};
use haloumi_ir_base::{
    cmp::CmpOp,
    equivalency::{EqvRelation, SymbolicEqv},
    felt::Felt,
};
use haloumi_lowering::{
    Lowering,
    lowerable::{LowerableExpr, LowerableStmt},
};

pub struct Constraint<T> {
    op: CmpOp,
    lhs: T,
    rhs: T,
}

impl<T> Constraint<T> {
    pub fn new(op: CmpOp, lhs: T, rhs: T) -> Self {
        Self { op, lhs, rhs }
    }

    pub fn map<O>(self, f: &impl Fn(T) -> O) -> Constraint<O> {
        Constraint::new(self.op, f(self.lhs), f(self.rhs))
    }

    pub fn map_into<O>(&self, f: &impl Fn(&T) -> O) -> Constraint<O> {
        Constraint::new(self.op, f(&self.lhs), f(&self.rhs))
    }

    pub fn try_map<O, E>(self, f: &impl Fn(T) -> Result<O, E>) -> Result<Constraint<O>, E> {
        Ok(Constraint::new(self.op, f(self.lhs)?, f(self.rhs)?))
    }

    pub fn try_map_inplace<E>(&mut self, f: &impl Fn(&mut T) -> Result<(), E>) -> Result<(), E> {
        f(&mut self.lhs)?;
        f(&mut self.rhs)
    }

    pub fn op(&self) -> CmpOp {
        self.op
    }

    pub fn lhs(&self) -> &T {
        &self.lhs
    }

    pub fn rhs(&self) -> &T {
        &self.rhs
    }

    pub fn rhs_mut(&mut self) -> &mut T {
        &mut self.rhs
    }

    pub fn lhs_mut(&mut self) -> &mut T {
        &mut self.lhs
    }
}

impl<T> From<Constraint<T>> for IRBexpr<T> {
    fn from(value: Constraint<T>) -> Self {
        IRBexpr::Cmp(value.op, value.lhs, value.rhs)
    }
}

impl Constraint<IRAexpr> {
    /// Folds the statements if the expressions are constant.
    /// If a assert-like statement folds into a tautology (i.e. `(= 0 0 )`) gets removed. If it
    /// folds into a unsatisfiable proposition the method returns an error.
    pub fn constant_fold(&mut self, prime: Felt) -> Result<Option<IRStmt<IRAexpr>>, Error> {
        self.lhs.constant_fold(prime);
        self.rhs.constant_fold(prime);
        if let Some((lhs, rhs)) = self.lhs.const_value().zip(self.rhs.const_value()) {
            let r = match self.op {
                CmpOp::Eq => lhs == rhs,
                CmpOp::Lt => lhs < rhs,
                CmpOp::Le => lhs <= rhs,
                CmpOp::Gt => lhs > rhs,
                CmpOp::Ge => lhs >= rhs,
                CmpOp::Ne => lhs != rhs,
            };
            if r {
                return Ok(Some(IRStmt::empty()));
            } else {
                return Err(Error::FoldedFalseStmt("constraint", self.clone().into()));
            }
        }
        Ok(None)
    }

    /// Matches the statements against a series of known patterns and applies rewrites if able to.
    pub(crate) fn canonicalize(&mut self) {
        if let Some((op, lhs, rhs)) = canonicalize_constraint(self.op, &self.lhs, &self.rhs) {
            *self = Self::new(op, lhs, rhs);
        }
    }
}

impl<T: LowerableExpr> LowerableStmt for Constraint<T> {
    fn lower<L>(self, l: &L) -> haloumi_lowering::Result<()>
    where
        L: Lowering + ?Sized,
    {
        let lhs = self.lhs.lower(l)?;
        log::trace!("Lowered lhs");
        let rhs = self.rhs.lower(l)?;
        log::trace!("Lowered rhs");
        l.checked_generate_constraint(self.op, &lhs, &rhs)?;
        log::trace!("Lowered constraint");
        Ok(())
    }
}

impl<T: Clone> Clone for Constraint<T> {
    fn clone(&self) -> Self {
        Self {
            op: self.op,
            lhs: self.lhs.clone(),
            rhs: self.rhs.clone(),
        }
    }
}

impl<T: PartialEq> PartialEq for Constraint<T> {
    fn eq(&self, other: &Self) -> bool {
        self.op == other.op && self.lhs == other.lhs && self.rhs == other.rhs
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Constraint<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{:#?} {} {:#?}", self.lhs, self.op, self.rhs)
        } else {
            write!(f, "{:?} {} {:?}", self.lhs, self.op, self.rhs)
        }
    }
}

impl<L, R> EqvRelation<Constraint<L>, Constraint<R>> for SymbolicEqv
where
    SymbolicEqv: EqvRelation<L, R>,
{
    /// Two constraint statements are equivalent if they have the same operator and each side is
    /// equivalent to the other.
    fn equivalent(lhs: &Constraint<L>, rhs: &Constraint<R>) -> bool {
        lhs.op == rhs.op
            && <SymbolicEqv as EqvRelation<L, R>>::equivalent(&lhs.lhs, &rhs.lhs)
            && <SymbolicEqv as EqvRelation<L, R>>::equivalent(&lhs.rhs, &rhs.rhs)
    }
}

#[cfg(test)]
mod test {

    use crate::stmt::test::TestHelper;

    use super::*;
    use haloumi_core::table::RotationExt;

    #[test]
    fn test_partial_eq_on_i32() {
        let h = TestHelper::<i32, Constraint<i32>>::constraints();
        h.helper(1, 2, 3, 4);
    }

    mod ff {

        use super::*;
        use crate::stmt::test::ff::{MockExpr, a, c, f, i};
        use haloumi_core::table::Rotation;

        #[test]
        fn test_partial_eq_on_expressions() {
            let h = TestHelper::<MockExpr, Constraint<MockExpr>>::constraints();

            use Rotation as R;
            h.helper_with(|| c(1), || c(2), || c(3), || c(4));
            h.helper_with(|| f(1, R::cur()), || c(2), || c(3), || c(4));
            h.helper_with(|| a(1, R::cur()), || c(2), || c(3), || c(4));
            h.helper_with(|| i(1, R::cur()), || c(2), || c(3), || c(4));
        }

        #[test]
        fn test_partial_eq_on_row_expressions() {
            let h = TestHelper::<(usize, MockExpr), Constraint<(usize, MockExpr)>>::constraints();
            use Rotation as R;

            let x = || (0, a(0, R::cur()));
            let y = || {
                let f0_0 = f(0, R::cur());
                let a1_0 = a(1, R::cur());
                let a0_1 = a(0, R::next());
                (0, f0_0 * a1_0 + a0_1)
            };
            h.helper_with(x, y, || (0, c(3)), || (0, c(4)));
        }
    }
}
