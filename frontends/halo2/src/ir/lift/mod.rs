use std::{
    fmt,
    iter::{Product, Sum},
    marker::PhantomData,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    sync::{Mutex, MutexGuard},
};

use inner::LiftInner;
use lazy::{
    lazy_init_delta, lazy_init_mult_gen, lazy_init_one, lazy_init_root, lazy_init_root_inv,
    lazy_init_two_inv, lazy_init_zero,
};
use subtle::{Choice, ConditionallySelectable, ConstantTimeEq, CtOption};
use unwrapped::Unwrapped;

use crate::{
    arena::{BumpArena, Index},
    halo2::{Field, PrimeField},
};

mod inner;
mod lazy;
mod unwrapped;

lazy_static::lazy_static! {
    pub static ref LIFT_EXPR_ARENA: Mutex<BumpArena> = Mutex::new(BumpArena::new());
}

macro_rules! arena {
    ($cb:expr) => {{
        let mut arena = LIFT_EXPR_ARENA.lock().unwrap();
        $cb(&mut arena)
    }};
}

#[derive(Clone, Copy)]
pub enum Lift<F> {
    Assigned {
        index: Index,
        _marker: PhantomData<F>,
    },
    Zero,
    One,
    TwoInv,
    MultiplicativeGenerator,
    RootOfUnity,
    RootOfUnityInv,
    Delta,
}

impl<F> From<Index> for Lift<F> {
    fn from(index: Index) -> Self {
        Self::Assigned {
            index,
            _marker: Default::default(),
        }
    }
}

impl<F: PrimeField> Lift<F> {
    fn unwrap<'a, 'b: 'a>(&self, arena: &'b mut MutexGuard<BumpArena>) -> Unwrapped<'a, F> {
        match self.canonicalize_in_arena(arena) {
            Lift::Assigned { index, .. } => Unwrapped::new(arena.get(&index)),
            _ => unreachable!(),
        }
    }

    pub fn evaluate<T>(
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
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            self.unwrap(arena).evaluate(
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
        })
    }

    pub fn lift(id: usize) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| { arena.insert(LiftInner::lift(id)).into() })
    }

    pub fn canonicalize(&self) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| { self.canonicalize_in_arena(arena) })
    }

    fn canonicalize_in_arena(&self, arena: &mut MutexGuard<BumpArena>) -> Self {
        match self {
            s @ Lift::Assigned { .. } => s.clone(),
            Lift::Zero => lazy_init_zero::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::One => lazy_init_one::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::TwoInv => lazy_init_two_inv::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::MultiplicativeGenerator => {
                lazy_init_mult_gen::<F, Self>(arena, |_, idx| (*idx).into())
            }
            Lift::RootOfUnity => lazy_init_root::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::RootOfUnityInv => lazy_init_root_inv::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::Delta => lazy_init_delta::<F, Self>(arena, |_, idx| (*idx).into()),
        }
    }

    fn unwrap_many<'a, 'b: 'a, const N: usize>(
        values: [&Self; N],
        arena: &'b mut MutexGuard<BumpArena>,
    ) -> [Unwrapped<'a, F>; N] {
        let assigned = values
            .into_iter()
            .map(|s| s.canonicalize_in_arena(arena))
            .collect::<Vec<_>>();

        assigned
            .into_iter()
            .map(|f| match f {
                Lift::Assigned { index, .. } => Unwrapped::new(arena.get(&index)),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

impl<F: PrimeField + 'static> ConstantTimeEq for Lift<F> {
    fn ct_eq(&self, other: &Self) -> Choice {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([self, other], arena);
            lhs.ct_eq(&rhs)
        })
    }
}

impl<F: PrimeField> PartialEq for Lift<F> {
    fn eq(&self, other: &Self) -> bool {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([self, other], arena);
            lhs.eq(&rhs)
        })
    }
}

impl<F: PrimeField> Neg for Lift<F> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let n = LiftInner::neg(self.unwrap(arena));
            arena.insert(n).into()
        })
    }
}

impl<F: PrimeField> Add for Lift<F> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([&self, &rhs], arena);
            let r = LiftInner::add(lhs, rhs);
            arena.insert(r).into()
        })
    }
}

impl<F: PrimeField> Sub for Lift<F> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([&self, &rhs], arena);
            let r = LiftInner::sub(lhs, rhs);
            arena.insert(r).into()
        })
    }
}

impl<F: PrimeField> Mul for Lift<F> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([&self, &rhs], arena);
            let r = LiftInner::mul(lhs, rhs);
            arena.insert(r).into()
        })
    }
}

impl<F: Sum + Add<Output = F> + Default + PrimeField> Sum for Lift<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, f| acc + f)
    }
}

impl<'a, F: PrimeField + 'static> Sum<&'a Self> for Lift<F> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, f| acc + f)
    }
}

impl<F: Default + 'static> Lift<F> {
    fn zero() -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            arena.insert(LiftInner::r#const(F::default())).into()
        })
    }
}

impl<F: Product + Mul<Output = F> + Default + PrimeField> Product for Lift<F> {
    fn product<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        match iter.next() {
            Some(it) => iter.fold(it, |acc, f| acc * f),
            None => Self::zero(),
        }
    }
}

impl<'a, F: PrimeField + 'static> Product<&'a Self> for Lift<F> {
    fn product<I: Iterator<Item = &'a Self>>(mut iter: I) -> Self {
        match iter.next() {
            Some(it) => iter.fold(it.clone(), |acc, f| acc * f),
            None => Default::default(),
        }
    }
}

impl<F: PrimeField> AddAssign for Lift<F> {
    fn add_assign(&mut self, rhs: Self) {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([self, &rhs], arena);
            let r = LiftInner::add(lhs, rhs);
            *self = arena.insert(r).into()
        });
    }
}

impl<'a, F: PrimeField> AddAssign<&'a Self> for Lift<F> {
    fn add_assign(&mut self, rhs: &'a Self) {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([self, &rhs], arena);
            let r = LiftInner::add(lhs, rhs);
            *self = arena.insert(r).into()
        });
    }
}

impl<F: PrimeField> SubAssign for Lift<F> {
    fn sub_assign(&mut self, rhs: Self) {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([self, &rhs], arena);
            let r = LiftInner::sub(lhs, rhs);
            *self = arena.insert(r).into()
        });
    }
}

impl<'a, F: PrimeField> SubAssign<&'a Self> for Lift<F> {
    fn sub_assign(&mut self, rhs: &'a Self) {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([self, &rhs], arena);
            let r = LiftInner::sub(lhs, rhs);
            *self = arena.insert(r).into()
        });
    }
}

impl<F: PrimeField> MulAssign for Lift<F> {
    fn mul_assign(&mut self, rhs: Self) {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([self, &rhs], arena);
            let r = LiftInner::mul(lhs, rhs);
            *self = arena.insert(r).into()
        });
    }
}

impl<'a, F: PrimeField> MulAssign<&'a Self> for Lift<F> {
    fn mul_assign(&mut self, rhs: &'a Self) {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([self, rhs], arena);
            let r = LiftInner::mul(lhs, rhs);
            *self = arena.insert(r).into()
        });
    }
}

impl<'a, F: PrimeField> Add<&'a Self> for Lift<F> {
    type Output = Self;

    fn add(self, rhs: &'a Self) -> Self::Output {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([&self, rhs], arena);
            let r = LiftInner::add(lhs, rhs);
            arena.insert(r).into()
        })
    }
}

impl<'a, F: PrimeField> Sub<&'a Self> for Lift<F> {
    type Output = Self;

    fn sub(self, rhs: &'a Self) -> Self::Output {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([&self, rhs], arena);
            let r = LiftInner::sub(lhs, rhs);
            arena.insert(r).into()
        })
    }
}

impl<'a, F: PrimeField> Mul<&'a Self> for Lift<F> {
    type Output = Self;

    fn mul(self, rhs: &'a Self) -> Self::Output {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([&self, &rhs], arena);
            let r = LiftInner::mul(lhs, rhs);
            arena.insert(r).into()
        })
    }
}

impl<F: Default + 'static> Default for Lift<F> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<F: PrimeField> ConditionallySelectable for Lift<F> {
    fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many([a, b], arena);
            let r = LiftInner::cond_sel(choice.into(), lhs, rhs);
            arena.insert(r).into()
        })
    }
}

impl<F: fmt::Debug> fmt::Debug for Lift<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lift::Assigned { index, .. } => {
                arena!(|arena: &mut MutexGuard<BumpArena>| {
                    write!(f, "Lift<F> {{ index = {:?},  inner = ", index)?;
                    match arena.try_get::<LiftInner>(index) {
                        Some(inner) => write!(f, "{inner:?}"),
                        None => write!(f, "<Not found>"),
                    }?;
                    write!(f, " }}")
                })
            }
            Lift::Zero => write!(f, "Lift<F> {{ Zero }}"),
            Lift::One => write!(f, "Lift<F> {{ One }}"),
            Lift::TwoInv => write!(f, "Lift<F> {{ TwoInv }}"),
            Lift::MultiplicativeGenerator => write!(f, "Lift<F> {{ MultiplicativeGenerator }}"),
            Lift::RootOfUnity => write!(f, "Lift<F> {{ RootOfUnity }}"),
            Lift::RootOfUnityInv => write!(f, "Lift<F> {{ RootOfUnityInv }}"),
            Lift::Delta => write!(f, "Lift<F> {{ Delta }}"),
        }
    }
}

impl<F: Eq> Eq for Lift<F> where Self: PartialEq {}

impl<F: PrimeField> Field for Lift<F>
where
    Self: PartialEq,
{
    const ZERO: Self = Self::Zero;

    const ONE: Self = Self::One;

    fn random(rng: impl rand::RngCore) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            arena.insert(LiftInner::r#const(F::random(rng))).into()
        })
    }

    fn square(&self) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let r = LiftInner::square(self.unwrap(arena));
            arena.insert(r).into()
        })
    }

    fn double(&self) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let r = LiftInner::double(self.unwrap(arena));
            arena.insert(r).into()
        })
    }

    fn invert(&self) -> CtOption<Self> {
        let r = arena!(|arena: &mut MutexGuard<BumpArena>| {
            let r = LiftInner::inv(self.unwrap(arena));
            arena.insert(r).into()
        });
        CtOption::new(r, if *self == Self::zero() { 0 } else { 1 }.into())
    }

    fn sqrt_ratio(num: &Self, div: &Self) -> (Choice, Self) {
        (
            1.into(),
            arena!(|arena: &mut MutexGuard<BumpArena>| {
                let [lhs, rhs] = Self::unwrap_many([num, div], arena);
                let r = LiftInner::sqrt_ratio(lhs, rhs);
                arena.insert(r).into()
            }),
        )
    }
}

#[derive(Copy, Clone, Default)]
pub struct LiftRepr<F: PrimeField>(PhantomData<F>);

impl<F: PrimeField> AsRef<[u8]> for LiftRepr<F> {
    fn as_ref(&self) -> &[u8] {
        unreachable!()
    }
}

impl<F: PrimeField> AsMut<[u8]> for LiftRepr<F> {
    fn as_mut(&mut self) -> &mut [u8] {
        unreachable!()
    }
}

impl<F: From<u64> + 'static> From<u64> for Lift<F> {
    fn from(value: u64) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            arena.insert(LiftInner::r#const(F::from(value))).into()
        })
    }
}

impl<F: PrimeField> PrimeField for Lift<F> {
    type Repr = LiftRepr<F>;

    fn from_repr(_repr: Self::Repr) -> CtOption<Self> {
        unreachable!()
    }

    fn to_repr(&self) -> Self::Repr {
        unreachable!()
    }

    fn is_odd(&self) -> Choice {
        0.into()
    }

    const MODULUS: &'static str = F::MODULUS;

    const NUM_BITS: u32 = F::NUM_BITS;

    const CAPACITY: u32 = F::CAPACITY;

    const TWO_INV: Self = Self::TwoInv;

    const MULTIPLICATIVE_GENERATOR: Self = Self::MultiplicativeGenerator;

    const S: u32 = F::S;

    const ROOT_OF_UNITY: Self = Self::RootOfUnity;

    const ROOT_OF_UNITY_INV: Self = Self::RootOfUnityInv;

    const DELTA: Self = Self::Delta;
}
