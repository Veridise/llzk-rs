use std::{any::Any, rc::Rc};

use super::unwrapped::Unwrapped;

#[derive(Clone, Debug)]
pub struct InnerConst(Rc<Box<dyn Any>>);

impl InnerConst {
    pub fn new<F: 'static>(value: F) -> Self {
        Self(Rc::new(Box::new(value)))
    }
    pub fn as_f<F: Clone + 'static>(&self) -> F {
        self.0
            .downcast_ref::<F>()
            .expect("const is not of the expected type")
            .clone()
    }

    pub fn try_as_f<F: Clone + 'static>(&self) -> Option<F> {
        self.0.downcast_ref::<F>().map(|f| f.clone())
    }
}

#[derive(Clone, Debug)]
pub enum LiftInner {
    Const(InnerConst),
    Lift(usize),
    Add(Box<LiftInner>, Box<LiftInner>),
    Sub(Box<LiftInner>, Box<LiftInner>),
    Mul(Box<LiftInner>, Box<LiftInner>),
    Neg(Box<LiftInner>),
    Square(Box<LiftInner>),
    Double(Box<LiftInner>),
    Invert(Box<LiftInner>),
    SqrtRatio(Box<LiftInner>, Box<LiftInner>),
    CondSelect(bool, Box<LiftInner>, Box<LiftInner>),
}

impl LiftInner {
    pub fn evaluate<F: Clone + 'static, T>(
        &self,
        constant: &impl Fn(F) -> T,
        lift: &impl Fn(usize) -> T,
        add: &impl Fn(T, T) -> T,
        sub: &impl Fn(T, T) -> T,
        mul: &impl Fn(T, T) -> T,
        neg: &impl Fn(T) -> T,
        square: &impl Fn(T) -> T,
        double: &impl Fn(T) -> T,
        invert: &impl Fn(T) -> T,
        sqrt_ratio: &impl Fn(T, T) -> T,
        cond_select: &impl Fn(bool, T, T) -> T,
    ) -> T {
        let eval = |e: &LiftInner| {
            e.evaluate(
                constant,
                lift,
                add,
                sub,
                mul,
                neg,
                square,
                double,
                invert,
                sqrt_ratio,
                cond_select,
            )
        };
        match self {
            LiftInner::Const(i) => constant(i.as_f()),
            LiftInner::Lift(id) => lift(*id),
            LiftInner::Add(lhs, rhs) => add(eval(lhs), eval(rhs)),
            LiftInner::Sub(lhs, rhs) => sub(eval(lhs), eval(rhs)),
            LiftInner::Mul(lhs, rhs) => mul(eval(lhs), eval(rhs)),
            LiftInner::Neg(expr) => neg(eval(expr)),
            LiftInner::Square(expr) => square(eval(expr)),
            LiftInner::Double(expr) => double(eval(expr)),
            LiftInner::Invert(expr) => invert(eval(expr)),
            LiftInner::SqrtRatio(lhs, rhs) => sqrt_ratio(eval(lhs), eval(rhs)),
            LiftInner::CondSelect(cond, lhs, rhs) => cond_select(*cond, eval(lhs), eval(rhs)),
        }
    }

    pub fn lift(id: usize) -> Self {
        Self::Lift(id)
    }

    pub fn r#const<'a, F: 'static>(f: F) -> Self {
        Self::Const(InnerConst::new(f))
    }

    pub fn neg<'a, F>(w: Unwrapped<'a, F>) -> Self {
        Self::Neg(w.boxed())
    }
    pub fn square<'a, F>(w: Unwrapped<'a, F>) -> Self {
        Self::Square(w.boxed())
    }
    pub fn double<'a, F>(w: Unwrapped<'a, F>) -> Self {
        Self::Double(w.boxed())
    }
    pub fn inv<'a, F>(w: Unwrapped<'a, F>) -> Self {
        Self::Invert(w.boxed())
    }

    pub fn add<'a, 'b, F>(lhs: Unwrapped<'a, F>, rhs: Unwrapped<'b, F>) -> Self {
        Self::Add(lhs.boxed(), rhs.boxed())
    }
    pub fn sub<'a, 'b, F>(lhs: Unwrapped<'a, F>, rhs: Unwrapped<'b, F>) -> Self {
        Self::Sub(lhs.boxed(), rhs.boxed())
    }
    pub fn mul<'a, 'b, F>(lhs: Unwrapped<'a, F>, rhs: Unwrapped<'b, F>) -> Self {
        Self::Mul(lhs.boxed(), rhs.boxed())
    }
    pub fn sqrt_ratio<'a, 'b, F>(lhs: Unwrapped<'a, F>, rhs: Unwrapped<'b, F>) -> Self {
        Self::SqrtRatio(lhs.boxed(), rhs.boxed())
    }
    pub fn cond_sel<'a, 'b, F>(cond: bool, lhs: Unwrapped<'a, F>, rhs: Unwrapped<'b, F>) -> Self {
        Self::CondSelect(cond, lhs.boxed(), rhs.boxed())
    }
}
