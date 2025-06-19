use std::{
    fmt,
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use subtle::{Choice, ConditionallySelectable, ConstantTimeEq, CtOption};

use crate::halo2::{Field, Value};

/// IR for operations that occur in the main circuit.
pub enum CircuitStmt<T> {
    ConstraintCall(String, Vec<Value<T>>, Vec<Value<T>>),
    EqConstraint(Value<T>, Value<T>),
}

pub type CircuitStmts<T> = Vec<CircuitStmt<T>>;

pub enum Lift<F> {
    Unk,
    Lift,
    Const(F),
}

impl<F: Copy> Copy for Lift<F> {}

impl<F: ConstantTimeEq> ConstantTimeEq for Lift<F> {
    fn ct_eq(&self, other: &Self) -> Choice {
        match (self, other) {
            (Lift::Unk, Lift::Unk) | (Lift::Lift, Lift::Lift) => 1.into(),
            (Lift::Const(lhs), Lift::Const(rhs)) => lhs.ct_eq(rhs),
            _ => 0.into(),
        }
    }
}

impl<F: Neg<Output = F>> Neg for Lift<F> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self.move_map(|f| f.neg()).into()
    }
}

impl<F: Add<Output = F>> Add for Lift<F> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Lift::Const(lhs), Lift::Const(rhs)) => (lhs + rhs).into(),
            _ => Lift::Unk,
        }
    }
}

impl<F: Sub<Output = F>> Sub for Lift<F> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Lift::Const(lhs), Lift::Const(rhs)) => (lhs - rhs).into(),
            _ => Lift::Unk,
        }
    }
}

impl<F: Mul<Output = F>> Mul for Lift<F> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Lift::Const(lhs), Lift::Const(rhs)) => (lhs * rhs).into(),
            _ => Lift::Unk,
        }
    }
}

impl<F: Sum + Add<Output = F> + Default> Sum for Lift<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, f| acc + f)
    }
}

impl<'a, F: Sum<&'a F> + Add<&'a F, Output = F> + Default> Sum<&'a Self> for Lift<F> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, f| acc + f)
    }
}

impl<F: Product + Mul<Output = F> + Default> Product for Lift<F> {
    fn product<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        match iter.next() {
            Some(it) => iter.fold(it, |acc, f| acc * f),
            None => Default::default(),
        }
    }
}

impl<'a, F: Product<&'a F> + Mul<&'a F, Output = F> + Clone + Default> Product<&'a Self>
    for Lift<F>
{
    fn product<I: Iterator<Item = &'a Self>>(mut iter: I) -> Self {
        match iter.next() {
            Some(it) => iter.fold(it.clone(), |acc, f| acc * f),
            None => Default::default(),
        }
    }
}

impl<F: AddAssign> AddAssign for Lift<F> {
    fn add_assign(&mut self, rhs: Self) {
        if let Some(f) = self.as_const_mut() {
            if let Lift::Const(rhs) = rhs {
                f.add_assign(rhs);
            }
        } else {
            *self = Lift::Unk;
        }
    }
}

impl<'a, F: AddAssign<&'a F>> AddAssign<&'a Self> for Lift<F> {
    fn add_assign(&mut self, rhs: &'a Self) {
        if let Some(f) = self.as_const_mut() {
            if let Lift::Const(rhs) = rhs {
                f.add_assign(rhs);
            }
        } else {
            *self = Lift::Unk;
        }
    }
}

impl<F: SubAssign> SubAssign for Lift<F> {
    fn sub_assign(&mut self, rhs: Self) {
        if let Some(f) = self.as_const_mut() {
            if let Lift::Const(rhs) = rhs {
                f.sub_assign(rhs);
            }
        } else {
            *self = Lift::Unk;
        }
    }
}

impl<'a, F: SubAssign<&'a F>> SubAssign<&'a Self> for Lift<F> {
    fn sub_assign(&mut self, rhs: &'a Self) {
        if let Some(f) = self.as_const_mut() {
            if let Lift::Const(rhs) = rhs {
                f.sub_assign(rhs);
            }
        } else {
            *self = Lift::Unk;
        }
    }
}

impl<F: MulAssign> MulAssign for Lift<F> {
    fn mul_assign(&mut self, rhs: Self) {
        if let Some(f) = self.as_const_mut() {
            if let Lift::Const(rhs) = rhs {
                f.mul_assign(rhs);
            }
        } else {
            *self = Lift::Unk;
        }
    }
}

impl<'a, F: MulAssign<&'a F>> MulAssign<&'a Self> for Lift<F> {
    fn mul_assign(&mut self, rhs: &'a Self) {
        if let Some(f) = self.as_const_mut() {
            if let Lift::Const(rhs) = rhs {
                f.mul_assign(rhs);
            }
        } else {
            *self = Lift::Unk;
        }
    }
}

impl<'a, F: Add<&'a F, Output = F>> Add<&'a Self> for Lift<F> {
    type Output = Self;

    fn add(self, rhs: &'a Self) -> Self::Output {
        match (self, rhs) {
            (Lift::Const(lhs), Lift::Const(rhs)) => (lhs + rhs).into(),
            _ => Lift::Unk,
        }
    }
}

impl<'a, F: Sub<&'a F, Output = F>> Sub<&'a Self> for Lift<F> {
    type Output = Self;

    fn sub(self, rhs: &'a Self) -> Self::Output {
        match (self, rhs) {
            (Lift::Const(lhs), Lift::Const(rhs)) => (lhs - rhs).into(),
            _ => Lift::Unk,
        }
    }
}

impl<'a, F: Mul<&'a F, Output = F>> Mul<&'a Self> for Lift<F> {
    type Output = Self;

    fn mul(self, rhs: &'a Self) -> Self::Output {
        match (self, rhs) {
            (Lift::Const(lhs), Lift::Const(rhs)) => (lhs * rhs).into(),
            _ => Lift::Unk,
        }
    }
}

impl<F: Default> Default for Lift<F> {
    fn default() -> Self {
        Lift::Const(Default::default())
    }
}

impl<F: ConditionallySelectable> ConditionallySelectable for Lift<F> {
    fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        match (a, b) {
            (Lift::Const(a), Lift::Const(b)) => F::conditional_select(a, b, choice).into(),
            _ => Lift::Unk,
        }
    }
}

impl<F: fmt::Debug> fmt::Debug for Lift<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lift::Unk => write!(f, "Unk"),
            Lift::Lift => write!(f, "Lift"),
            Lift::Const(v) => write!(f, "{v:?}"),
        }
    }
}

impl<F: Clone> Clone for Lift<F> {
    fn clone(&self) -> Self {
        match self {
            Lift::Const(f) => Lift::Const(f.clone()),
            Lift::Unk => Lift::Unk,
            Lift::Lift => Lift::Lift,
        }
    }
}

impl<F: PartialEq> PartialEq for Lift<F> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Lift::Unk, Lift::Unk) => true,
            (Lift::Lift, Lift::Lift) => true,
            (Lift::Const(lhs), Lift::Const(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl<F: Eq> Eq for Lift<F> {}

impl<F> Lift<F> {
    fn as_const(&self) -> Option<&F> {
        match self {
            Lift::Const(f) => Some(f),
            _ => None,
        }
    }

    fn as_const_mut(&mut self) -> Option<&mut F> {
        match self {
            Lift::Const(f) => Some(f),
            _ => None,
        }
    }

    fn move_as_const(self) -> Option<F> {
        match self {
            Lift::Const(f) => Some(f),
            _ => None,
        }
    }

    fn map<FN>(&self, f: FN) -> Self
    where
        FN: Fn(&F) -> F,
    {
        self.as_const().map(f).into()
    }

    fn move_map<FN>(self, f: FN) -> Self
    where
        FN: Fn(F) -> F,
    {
        self.move_as_const().map(f).into()
    }

    pub fn is_lift(&self) -> bool {
        match self {
            Self::Lift => true,
            _ => false,
        }
    }
}

impl<F> From<Option<F>> for Lift<F> {
    fn from(value: Option<F>) -> Self {
        match value {
            Some(f) => Self::Const(f),
            None => Self::Unk,
        }
    }
}

impl<F> From<F> for Lift<F> {
    fn from(f: F) -> Self {
        Self::Const(f)
    }
}

impl<F: Field> Field for Lift<F> {
    const ZERO: Self = Self::Const(F::ZERO);

    const ONE: Self = Self::Const(F::ONE);

    fn random(rng: impl rand::RngCore) -> Self {
        Self::Const(F::random(rng))
    }

    fn square(&self) -> Self {
        self.map(|f| f.square())
    }

    fn double(&self) -> Self {
        self.map(|f| f.double())
    }

    fn invert(&self) -> CtOption<Self> {
        match self.as_const().map(|f| f.invert()) {
            Some(f) => f.map(From::from),
            None => CtOption::new(self.clone(), 1.into()),
        }
    }

    fn sqrt_ratio(num: &Self, div: &Self) -> (Choice, Self) {
        match (num, div) {
            (Lift::Const(num), Lift::Const(div)) => {
                let (c, s) = F::sqrt_ratio(num, div);
                (c, s.into())
            }
            _ => (1.into(), Lift::Unk),
        }
    }
}
