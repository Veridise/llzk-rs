use anyhow::Result;

use crate::{
    backend::lowering::{
        lowerable::{LowerableExpr, LowerableStmt},
        Lowering,
    },
    ir::{
        canon::canonicalize_constraint,
        equivalency::EqvRelation,
        expr::{Felt, IRAexpr},
        stmt::IRStmt,
        CmpOp,
    },
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

    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<Constraint<O>> {
        Ok(Constraint::new(self.op, f(self.lhs)?, f(self.rhs)?))
    }

    pub fn try_map_inplace(&mut self, f: &impl Fn(&mut T) -> Result<()>) -> Result<()> {
        f(&mut self.lhs)?;
        f(&mut self.rhs)
    }
}

impl Constraint<IRAexpr> {
    /// Folds the statements if the expressions are constant.
    /// If a assert-like statement folds into a tautology (i.e. `(= 0 0 )`) gets removed. If it
    /// folds into a unsatisfiable proposition the method returns an error.
    pub fn constant_fold(&mut self, prime: Felt) -> Result<Option<IRStmt<IRAexpr>>> {
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
                return Err(anyhow::anyhow!(
                    "Detected constraint statement with false predicate: {} {} {}",
                    lhs,
                    self.op,
                    rhs
                ));
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
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        l.checked_generate_constraint(self.op, &self.lhs.lower(l)?, &self.rhs.lower(l)?)
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

impl<L, R, E> EqvRelation<Constraint<L>, Constraint<R>> for E
where
    E: EqvRelation<L, R>,
{
    /// Two constraint statements are equivalent if they have the same operator and each side is
    /// equivalent to the other.
    fn equivalent(lhs: &Constraint<L>, rhs: &Constraint<R>) -> bool {
        lhs.op == rhs.op
            && <E as EqvRelation<L, R>>::equivalent(&lhs.lhs, &rhs.lhs)
            && <E as EqvRelation<L, R>>::equivalent(&lhs.rhs, &rhs.rhs)
    }
}

#[cfg(test)]
mod test {

    use crate::ir::stmt::test::TestHelper;

    use super::*;

    #[test]
    fn test_partial_eq_on_i32() {
        let h = TestHelper::<i32, Constraint<i32>>::constraints();
        h.helper(1, 2, 3, 4);
    }

    mod ff {
        use super::*;
        use crate::halo2::{ColumnType, Expression, Field, Fixed, Rotation};

        type F = crate::halo2::Fr;

        fn c(n: usize) -> Expression<F> {
            let one = F::ONE;
            let f = vec![one; n].into_iter().sum();
            Expression::Constant(f)
        }

        fn f(col: usize, rot: Rotation) -> Expression<F> {
            Fixed.query_cell(col, rot)
        }

        fn a(col: usize, rot: Rotation) -> Expression<F> {
            crate::halo2::Advice::default().query_cell(col, rot)
        }

        fn i(col: usize, rot: Rotation) -> Expression<F> {
            crate::halo2::Instance.query_cell(col, rot)
        }

        #[test]
        fn test_partial_eq_on_expressions() {
            let h = TestHelper::<Expression<F>, Constraint<Expression<F>>>::constraints();
            use Rotation as R;
            h.helper_with(|| c(1), || c(2), || c(3), || c(4));
            h.helper_with(|| f(1, R::cur()), || c(2), || c(3), || c(4));
            h.helper_with(|| a(1, R::cur()), || c(2), || c(3), || c(4));
            h.helper_with(|| i(1, R::cur()), || c(2), || c(3), || c(4));
        }

        #[test]
        fn test_partial_eq_on_row_expressions() {
            let h = TestHelper::<(usize, Expression<F>), Constraint<(usize, Expression<F>)>>::constraints();
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
