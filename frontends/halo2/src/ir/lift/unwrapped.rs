use std::marker::PhantomData;

use subtle::{Choice, ConstantTimeEq};

use super::inner::LiftInner;

#[derive(Debug)]
pub struct Unwrapped<'a, F> {
    inner: &'a LiftInner,
    _marker: PhantomData<F>,
}

impl<'a, F> Unwrapped<'a, F> {
    pub fn new(inner: &'a LiftInner) -> Self {
        Self {
            inner,
            _marker: Default::default(),
        }
    }

    pub fn boxed(&self) -> Box<LiftInner> {
        Box::new(self.inner.clone())
    }
}

impl<F: ConstantTimeEq + Clone + 'static> ConstantTimeEq for Unwrapped<'_, F> {
    fn ct_eq(&self, other: &Self) -> Choice {
        let b = match (self.inner, other.inner) {
            (LiftInner::Const(lhs), LiftInner::Const(rhs)) => {
                let lhs = lhs.try_as_f::<F>();
                let rhs = rhs.try_as_f::<F>();
                match (lhs, rhs) {
                    (Some(lhs), Some(rhs)) => lhs.ct_eq(&rhs).into(),
                    _ => false,
                }
            }
            (LiftInner::Lift, LiftInner::Lift) => true,
            (LiftInner::Add(l1, r1), LiftInner::Add(l2, r2)) => {
                ct_eq_box::<F>(l1, l2) && ct_eq_box::<F>(r1, r2)
            }
            (LiftInner::Sub(l1, r1), LiftInner::Sub(l2, r2)) => {
                ct_eq_box::<F>(l1, l2) && ct_eq_box::<F>(r1, r2)
            }
            (LiftInner::Mul(l1, r1), LiftInner::Mul(l2, r2)) => {
                ct_eq_box::<F>(l1, l2) && ct_eq_box::<F>(r1, r2)
            }
            (LiftInner::Neg(e1), LiftInner::Neg(e2)) => ct_eq_box::<F>(e1, e2),
            (LiftInner::Square(e1), LiftInner::Square(e2)) => ct_eq_box::<F>(e1, e2),
            (LiftInner::Double(e1), LiftInner::Double(e2)) => ct_eq_box::<F>(e1, e2),
            (LiftInner::Invert(e1), LiftInner::Invert(e2)) => ct_eq_box::<F>(e1, e2),
            (LiftInner::SqrtRatio(l1, r1), LiftInner::SqrtRatio(l2, r2)) => {
                ct_eq_box::<F>(l1, l2) && ct_eq_box::<F>(r1, r2)
            }
            _ => false,
        };
        if b { 1 } else { 0 }.into()
    }
}

impl<F: PartialEq + Clone + 'static> PartialEq for Unwrapped<'_, F> {
    fn eq(&self, other: &Self) -> bool {
        match (self.inner, other.inner) {
            (LiftInner::Const(lhs), LiftInner::Const(rhs)) => {
                let lhs = lhs.try_as_f::<F>();
                let rhs = rhs.try_as_f::<F>();
                match (lhs, rhs) {
                    (Some(lhs), Some(rhs)) => lhs.eq(&rhs).into(),
                    _ => false,
                }
            }
            (LiftInner::Lift, LiftInner::Lift) => true,
            (LiftInner::Add(l1, r1), LiftInner::Add(l2, r2)) => {
                eq_box::<F>(l1, l2) && eq_box::<F>(r1, r2)
            }
            (LiftInner::Sub(l1, r1), LiftInner::Sub(l2, r2)) => {
                eq_box::<F>(l1, l2) && eq_box::<F>(r1, r2)
            }
            (LiftInner::Mul(l1, r1), LiftInner::Mul(l2, r2)) => {
                eq_box::<F>(l1, l2) && eq_box::<F>(r1, r2)
            }
            (LiftInner::Neg(e1), LiftInner::Neg(e2)) => eq_box::<F>(e1, e2),
            (LiftInner::Square(e1), LiftInner::Square(e2)) => eq_box::<F>(e1, e2),
            (LiftInner::Double(e1), LiftInner::Double(e2)) => eq_box::<F>(e1, e2),
            (LiftInner::Invert(e1), LiftInner::Invert(e2)) => eq_box::<F>(e1, e2),
            (LiftInner::SqrtRatio(l1, r1), LiftInner::SqrtRatio(l2, r2)) => {
                eq_box::<F>(l1, l2) && eq_box::<F>(r1, r2)
            }
            _ => false,
        }
    }
}

fn ct_eq_box<F: ConstantTimeEq + Clone + 'static>(
    lhs: &Box<LiftInner>,
    rhs: &Box<LiftInner>,
) -> bool {
    let lhs = Unwrapped::<F>::new(lhs);
    let rhs = Unwrapped::<F>::new(rhs);
    lhs.ct_eq(&rhs).into()
}

fn eq_box<F: PartialEq + Clone + 'static>(lhs: &Box<LiftInner>, rhs: &Box<LiftInner>) -> bool {
    let lhs = Unwrapped::<F>::new(lhs);
    let rhs = Unwrapped::<F>::new(rhs);
    lhs.eq(&rhs).into()
}
