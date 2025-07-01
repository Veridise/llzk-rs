use std::{
    any::{type_name, Any, TypeId},
    ops::RangeFrom,
    rc::Rc,
};

use super::unwrapped::Unwrapped;
use std::sync::Mutex;

pub trait AsF<F: 'static>
where
    Self: 'static,
{
    fn as_f(&self) -> &F {
        self.try_as_f().expect(
            format!(
                "Failed to convert to '{}' ({:?}). Inner is '{}' ({:?})",
                type_name::<F>(),
                TypeId::of::<F>(),
                self.raw_name(),
                self.raw().type_id(),
            )
            .as_str(),
        )
    }

    fn raw<'a>(&'a self) -> &'a dyn Any;

    fn raw_name(&self) -> &'static str;

    fn try_as_f(&self) -> Option<&F>;
}

#[derive(Clone, Debug)]
pub struct InnerConst(Rc<Box<dyn Any>>, &'static str);

impl InnerConst {
    pub fn new<F: 'static>(value: F) -> Self {
        Self(Rc::new(Box::new(value)), type_name::<F>())
    }
}

impl<F: 'static> AsF<F> for InnerConst {
    fn try_as_f(&self) -> Option<&F> {
        self.0.downcast_ref::<F>()
    }

    fn raw<'a>(&'a self) -> &'a dyn Any {
        &self.0
    }

    fn raw_name(&self) -> &'static str {
        self.1
    }
}

impl<F: 'static, T: AsF<F>> AsF<F> for Option<T> {
    fn try_as_f(&self) -> Option<&F> {
        self.as_ref().map(T::try_as_f).flatten()
    }

    fn raw(&self) -> &dyn Any {
        unimplemented!()
    }

    fn raw_name(&self) -> &'static str {
        unimplemented!()
    }
}

#[derive(Clone, Debug)]
pub enum LiftInner {
    Const(InnerConst),
    Lift(usize, Option<InnerConst>),
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
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    pub fn evaluate<F: Clone + 'static, T>(
        &self,
        constant: &impl Fn(&F) -> T,
        lift: &impl Fn(usize, Option<&F>) -> T,
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
            LiftInner::Lift(id, f) => lift(*id, f.as_ref().map(|f| f.as_f())),
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

    pub fn lift() -> Self {
        Self::Lift(next_lift_id(), None)
    }

    pub fn lift_value<F: 'static>(f: F) -> Self {
        Self::Lift(next_lift_id(), Some(InnerConst::new(f)))
    }

    pub fn r#const<'a, F: 'static>(f: F) -> Self {
        Self::Const(InnerConst::new(f))
    }

    pub fn neg<'a, F: 'static>(w: Unwrapped<'a, F>) -> Self {
        Self::Neg(w.boxed())
    }
    pub fn square<'a, F: 'static>(w: Unwrapped<'a, F>) -> Self {
        Self::Square(w.boxed())
    }
    pub fn double<'a, F: 'static>(w: Unwrapped<'a, F>) -> Self {
        Self::Double(w.boxed())
    }
    pub fn inv<'a, F: 'static>(w: Unwrapped<'a, F>) -> Self {
        Self::Invert(w.boxed())
    }

    pub fn add<'a, 'b, F: 'static>(lhs: Unwrapped<'a, F>, rhs: Unwrapped<'b, F>) -> Self {
        Self::Add(lhs.boxed(), rhs.boxed())
    }
    pub fn sub<'a, 'b, F: 'static>(lhs: Unwrapped<'a, F>, rhs: Unwrapped<'b, F>) -> Self {
        Self::Sub(lhs.boxed(), rhs.boxed())
    }
    pub fn mul<'a, 'b, F: 'static>(lhs: Unwrapped<'a, F>, rhs: Unwrapped<'b, F>) -> Self {
        Self::Mul(lhs.boxed(), rhs.boxed())
    }
    pub fn sqrt_ratio<'a, 'b, F: 'static>(lhs: Unwrapped<'a, F>, rhs: Unwrapped<'b, F>) -> Self {
        Self::SqrtRatio(lhs.boxed(), rhs.boxed())
    }
    pub fn cond_sel<'a, 'b, F: 'static>(
        cond: bool,
        lhs: Unwrapped<'a, F>,
        rhs: Unwrapped<'b, F>,
    ) -> Self {
        Self::CondSelect(cond, lhs.boxed(), rhs.boxed())
    }
}

impl<F: 'static> From<(usize, Option<F>)> for LiftInner {
    fn from(value: (usize, Option<F>)) -> Self {
        LiftInner::Lift(value.0, value.1.map(InnerConst::new::<F>))
    }
}

fn next_lift_id() -> usize {
    static COUNTER: Mutex<RangeFrom<usize>> = Mutex::new(0..);

    let mut guard = COUNTER.lock().unwrap();
    guard.next().unwrap()
}
