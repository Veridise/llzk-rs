use std::{marker::PhantomData, ops::Deref};

use subtle::{Choice, ConstantTimeEq};

use super::inner::{AsF, LiftInner};

#[derive(Debug)]
pub struct Unwrapped<'a, F> {
    inner: &'a LiftInner,
    _marker: PhantomData<F>,
}

impl<'a, F> Deref for Unwrapped<'a, F> {
    type Target = LiftInner;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, F: 'static> Unwrapped<'a, F> {
    pub fn new(inner: &'a LiftInner) -> Self {
        Self {
            inner,
            _marker: Default::default(),
        }
    }

    pub fn boxed(&self) -> Box<LiftInner> {
        Box::new(self.inner.clone())
    }

    fn eq_impl<Feq, FeqO, Beq, BeqO>(&self, other: &Self, feq: &Feq, beq: &Beq) -> bool
    where
        Feq: Fn(&F, &F) -> FeqO,
        FeqO: Into<bool>,
        Beq: Fn(&Unwrapped<'_, F>, &Unwrapped<'_, F>) -> BeqO,
        BeqO: Into<bool>,
    {
        fn eq_box<F, FN, FO>(lhs: &LiftInner, rhs: &LiftInner, eq: &FN) -> bool
        where
            F: 'static,
            FN: Fn(&Unwrapped<'_, F>, &Unwrapped<'_, F>) -> FO,
            FO: Into<bool>,
        {
            let lhs = Unwrapped::<F>::new(lhs);
            let rhs = Unwrapped::<F>::new(rhs);
            eq(&lhs, &rhs).into()
        }

        fn as_f_eq<F: 'static, FN, FO>(lhs: &impl AsF<F>, rhs: &impl AsF<F>, eq: &FN) -> bool
        where
            FN: Fn(&F, &F) -> FO,
            FO: Into<bool>,
        {
            match (lhs.try_as_f(), rhs.try_as_f()) {
                (Some(lhs), Some(rhs)) => eq(lhs, rhs).into(),
                _ => false,
            }
        }

        match (self.inner, other.inner) {
            (LiftInner::Const(lhs), LiftInner::Const(rhs)) => as_f_eq(lhs, rhs, feq),
            (LiftInner::Lift(id1, f1), LiftInner::Lift(id2, f2)) => {
                id1 == id2 && as_f_eq(f1, f2, feq)
            }
            (LiftInner::Add(l1, r1), LiftInner::Add(l2, r2)) => {
                eq_box(l1, l2, beq) && eq_box(r1, r2, beq)
            }
            (LiftInner::Sub(l1, r1), LiftInner::Sub(l2, r2)) => {
                eq_box(l1, l2, beq) && eq_box(r1, r2, beq)
            }
            (LiftInner::Mul(l1, r1), LiftInner::Mul(l2, r2)) => {
                eq_box(l1, l2, beq) && eq_box(r1, r2, beq)
            }
            (LiftInner::Neg(e1), LiftInner::Neg(e2)) => eq_box(e1, e2, beq),
            (LiftInner::Square(e1), LiftInner::Square(e2)) => eq_box(e1, e2, beq),
            (LiftInner::Double(e1), LiftInner::Double(e2)) => eq_box(e1, e2, beq),
            (LiftInner::Invert(e1), LiftInner::Invert(e2)) => eq_box(e1, e2, beq),
            (LiftInner::SqrtRatio(l1, r1), LiftInner::SqrtRatio(l2, r2)) => {
                eq_box(l1, l2, beq) && eq_box(r1, r2, beq)
            }
            _ => false,
        }
    }
}

impl<F: ConstantTimeEq + Clone + 'static> ConstantTimeEq for Unwrapped<'_, F> {
    fn ct_eq(&self, other: &Self) -> Choice {
        if self.eq_impl(
            other,
            &|lhs: &F, rhs| lhs.ct_eq(rhs),
            &|lhs: &Unwrapped<'_, F>, rhs| lhs.ct_eq(rhs),
        ) {
            1
        } else {
            0
        }
        .into()
    }
}

impl<F: PartialEq + Clone + 'static> PartialEq for Unwrapped<'_, F> {
    fn eq(&self, other: &Self) -> bool {
        self.eq_impl(
            other,
            &|lhs: &F, rhs| lhs.eq(rhs),
            &|lhs: &Unwrapped<'_, F>, rhs| lhs.eq(rhs),
        )
    }
}
