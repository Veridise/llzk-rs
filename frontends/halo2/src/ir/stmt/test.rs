use helpers::{ConstraintFactory, IRStmtFactory, InnerConstraintFactory};

use super::*;

pub mod helpers;

type S = IRStmt<()>;

#[test]
fn iterator_nested_seqs() {
    let nested = S::seq([
        S::assert(IRBexpr::And(vec![])),
        S::seq([
            S::assert(IRBexpr::And(vec![])),
            S::assert(IRBexpr::And(vec![])),
        ]),
    ]);
    let expected = vec![S::assert(IRBexpr::And(vec![])); 3];
    let output = nested.into_iter().collect::<Vec<_>>();
    assert_eq!(expected, output);
}

fn eq<T>(lhs: T, rhs: T) -> IRStmt<T> {
    IRStmt::constraint(CmpOp::Eq, lhs, rhs)
}

fn lt<T>(lhs: T, rhs: T) -> IRStmt<T> {
    IRStmt::constraint(CmpOp::Lt, lhs, rhs)
}

pub struct TestHelper<T, O> {
    factory: Box<dyn ConstraintFactory<Inner = T, Out = O>>,
}

impl<T: 'static> TestHelper<T, Constraint<T>> {
    pub fn constraints() -> Self {
        Self {
            factory: Box::new(InnerConstraintFactory::default()),
        }
    }
}

impl<T: 'static> TestHelper<T, IRStmt<T>> {
    pub fn stmts() -> Self {
        Self {
            factory: Box::new(IRStmtFactory::default()),
        }
    }
}

impl<T, O> TestHelper<T, O>
where
    T: PartialEq + std::fmt::Debug + Clone,
    O: PartialEq + std::fmt::Debug,
{
    pub fn helper(&self, x: T, y: T, z: T, w: T) {
        let f = self.factory.as_ref();
        let a = f.eq(x.clone(), y.clone());
        let b = f.eq(x, y);
        let c = f.eq(z.clone(), w.clone());
        let d = f.lt(z, w);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(c, d);
    }

    pub fn helper_with(
        &self,
        x: impl Fn() -> T,
        y: impl Fn() -> T,
        z: impl Fn() -> T,
        w: impl Fn() -> T,
    ) {
        let f = self.factory.as_ref();
        let a = f.eq(x(), y());
        let b = f.eq(x(), y());
        let c = f.eq(z(), w());
        let d = f.lt(z(), w());

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(c, d);
    }
}

impl<T> TestHelper<T, IRStmt<T>>
where
    T: PartialEq + std::fmt::Debug + Clone,
{
    pub fn helper_seqs_with(&self, x: impl Fn() -> T, y: impl Fn() -> T) {
        let f = self.factory.as_ref();
        let a = f.eq(x(), y());
        let b = IRStmt::from_iter([f.eq(x(), y())]);

        assert_eq!(a, b);
    }
}

#[test]
fn test_partial_eq_on_i32() {
    let h = TestHelper::<i32, IRStmt<i32>>::stmts();
    h.helper(1, 2, 3, 4);
}

pub mod ff {
    use crate::{
        halo2::{ColumnType, Expression, Field, Fixed, Rotation},
        ir::stmt::{test::TestHelper, IRStmt},
    };

    type F = crate::halo2::Fr;

    pub fn c(n: usize) -> Expression<F> {
        let one = F::ONE;
        let f = vec![one; n].into_iter().sum();
        Expression::Constant(f)
    }

    pub fn f(col: usize, rot: Rotation) -> Expression<F> {
        Fixed.query_cell(col, rot)
    }

    pub fn a(col: usize, rot: Rotation) -> Expression<F> {
        crate::halo2::Advice::default().query_cell(col, rot)
    }

    pub fn i(col: usize, rot: Rotation) -> Expression<F> {
        crate::halo2::Instance.query_cell(col, rot)
    }

    #[test]
    fn test_partial_eq_on_expressions() {
        let h = TestHelper::<Expression<F>, IRStmt<Expression<F>>>::stmts();
        use Rotation as R;
        h.helper_with(|| c(1), || c(2), || c(3), || c(4));
        h.helper_with(|| f(1, R::cur()), || c(2), || c(3), || c(4));
        h.helper_with(|| a(1, R::cur()), || c(2), || c(3), || c(4));
        h.helper_with(|| i(1, R::cur()), || c(2), || c(3), || c(4));

        h.helper_seqs_with(|| c(1), || c(2));
        h.helper_seqs_with(|| f(1, R::cur()), || c(2));
        h.helper_seqs_with(|| a(1, R::cur()), || c(2));
        h.helper_seqs_with(|| i(1, R::cur()), || c(2));
    }
}
