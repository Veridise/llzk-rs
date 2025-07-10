use anyhow::Result;
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
    halo2::{Field, FromUniformBytes, PrimeField},
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

pub trait LiftLike: Sized + PrimeField {
    type Inner: PrimeField;

    #[allow(clippy::too_many_arguments)]
    fn evaluate<T>(
        &self,
        constant: &impl Fn(&Self::Inner) -> T,
        lift: &impl Fn(usize, Option<&Self::Inner>) -> T,
        add: &impl Fn(T, T) -> T,
        sub: &impl Fn(T, T) -> T,
        mul: &impl Fn(T, T) -> T,
        neg: &impl Fn(T) -> T,
        square: &impl Fn(T) -> T,
        double: &impl Fn(T) -> T,
        invert: &impl Fn(T) -> T,
        sqrt_ratio: &impl Fn(T, T) -> T,
        cond_select: &impl Fn(bool, T, T) -> T,
    ) -> T;

    fn simplify(&mut self) {
        *self = self.simplified();
    }
    fn simplified(&self) -> Self;

    fn concretized(&self) -> Option<Self::Inner>;

    fn is_symbolic(&self) -> bool {
        let or = |lhs, rhs| lhs || rhs;
        let ident = |e| e;
        self.evaluate(
            &|_| false,
            &|_, _| true,
            &or,
            &or,
            &or,
            &ident,
            &ident,
            &ident,
            &ident,
            &or,
            &|_, lhs, rhs| lhs || rhs,
        )
    }

    fn lift() -> Self;

    fn lift_value(f: Self::Inner) -> Self;

    fn lifted(self) -> Option<Self> {
        self.concretized().map(Self::lift_value)
    }

    fn canonicalize(&mut self) {
        *self = self.canonicalized();
    }
    fn canonicalized(&self) -> Self;
}

pub trait LiftLowering {
    type F: PrimeField;
    type Output;

    fn lower_constant(&self, f: &Self::F) -> Result<Self::Output>;

    fn lower_lifted(&self, id: usize, f: Option<&Self::F>) -> Result<Self::Output>;

    fn lower_add(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output>;

    fn lower_sub(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output>;

    fn lower_mul(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output>;

    fn lower_neg(&self, expr: &Self::Output) -> Result<Self::Output>;

    fn lower_double(&self, expr: &Self::Output) -> Result<Self::Output>;

    fn lower_square(&self, expr: &Self::Output) -> Result<Self::Output>;

    fn lower_invert(&self, expr: &Self::Output) -> Result<Self::Output>;

    fn lower_sqrt_ratio(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output>;

    fn lower_cond_select(
        &self,
        cond: bool,
        then: &Self::Output,
        r#else: &Self::Output,
    ) -> Result<Self::Output>;

    fn lower(
        &self,
        value: &impl LiftLike<Inner = Self::F>,
        simplify_first: bool,
    ) -> Result<Self::Output> {
        //arena!(|arena: &mut MutexGuard<BumpArena>| {
        if simplify_first {
            value.simplified()
        } else {
            *value
        }
        .evaluate(
            &|f| self.lower_constant(f),
            &|id, f| self.lower_lifted(id, f),
            &|lhs, rhs| self.lower_add(&lhs?, &rhs?),
            &|lhs, rhs| self.lower_sub(&lhs?, &rhs?),
            &|lhs, rhs| self.lower_mul(&lhs?, &rhs?),
            &|expr| self.lower_neg(&expr?),
            &|expr| self.lower_square(&expr?),
            &|expr| self.lower_double(&expr?),
            &|expr| self.lower_invert(&expr?),
            &|lhs, rhs| self.lower_sqrt_ratio(&lhs?, &rhs?),
            &|cond, lhs, rhs| self.lower_cond_select(cond, &lhs?, &rhs?),
        )
        //})
    }
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
    Const(F),
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
        match self.canonicalized_in_arena(arena) {
            Lift::Assigned { index, .. } => Unwrapped::new(arena.get(&index)),
            _ => unreachable!(),
        }
    }

    fn get_index(&self) -> Option<Index> {
        match self {
            Lift::Assigned { index, .. } => Some(*index),
            _ => None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn evaluate_in_arena<T>(
        &self,
        arena: &mut MutexGuard<BumpArena>,
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
    }

    pub const fn from_const(f: F) -> Self {
        Self::Const(f)
    }

    fn simplify_impl(
        &self,
        arena: &mut MutexGuard<BumpArena>,
        keep_lifted: bool,
    ) -> Result<F, LiftInner> {
        fn handle_binop<F, F1, F2>(
            lhs: Result<F, LiftInner>,
            rhs: Result<F, LiftInner>,
            f1: &F1,
            f2: &F2,
        ) -> Result<F, LiftInner>
        where
            F: 'static,
            F1: Fn(F, F) -> F,
            F2: Fn(Box<LiftInner>, Box<LiftInner>) -> LiftInner,
        {
            match (lhs, rhs) {
                (Ok(lhs), Ok(rhs)) => Ok(f1(lhs, rhs)),
                (Ok(lhs), Err(rhs)) => Err(f2(LiftInner::r#const(lhs).boxed(), rhs.boxed())),
                (Err(lhs), Ok(rhs)) => Err(f2(lhs.boxed(), LiftInner::r#const(rhs).boxed())),
                (Err(lhs), Err(rhs)) => Err(f2(lhs.boxed(), rhs.boxed())),
            }
        }

        fn handle_unary_op<F, F1, F2>(
            e: Result<F, LiftInner>,
            f1: &F1,
            f2: &F2,
        ) -> Result<F, LiftInner>
        where
            F: 'static,
            F1: Fn(&F) -> F,
            F2: Fn(Box<LiftInner>) -> LiftInner,
        {
            match e {
                Ok(f) => Ok(f1(&f)),
                Err(e) => Err(f2(e.boxed())),
            }
        }

        fn mk_binop_handler<F, F1, F2>(
            f1: &F1,
            f2: &F2,
        ) -> impl Fn(Result<F, LiftInner>, Result<F, LiftInner>) -> Result<F, LiftInner>
        where
            F: 'static,
            F1: Fn(F, F) -> F,
            F2: Fn(Box<LiftInner>, Box<LiftInner>) -> LiftInner,
        {
            |lhs, rhs| handle_binop::<F, F1, F2>(lhs, rhs, f1, f2)
        }

        fn mk_unop_handler<F, F1, F2>(
            f1: &F1,
            f2: &F2,
        ) -> impl Fn(Result<F, LiftInner>) -> Result<F, LiftInner>
        where
            F: 'static,
            F1: Fn(&F) -> F,
            F2: Fn(Box<LiftInner>) -> LiftInner,
        {
            |e| handle_unary_op::<F, F1, F2>(e, f1, f2)
        }

        self.evaluate_in_arena(
            arena,
            &|f| Ok(*f),
            &|id, f| match f {
                Some(inner) if !keep_lifted => Ok(*inner),
                _ => Err((id, f.copied()).into()),
            },
            &mk_binop_handler(&|lhs, rhs| lhs + rhs, &LiftInner::Add),
            &mk_binop_handler(&|lhs, rhs| lhs - rhs, &LiftInner::Sub),
            &mk_binop_handler(&|lhs, rhs| lhs * rhs, &LiftInner::Mul),
            &mk_unop_handler(&|f: &F| f.neg(), &LiftInner::Neg),
            &mk_unop_handler(&F::square, &LiftInner::Square),
            &mk_unop_handler(&F::double, &LiftInner::Double),
            &mk_unop_handler(&|f: &F| f.invert().unwrap_or(F::ZERO), &LiftInner::Invert),
            &mk_binop_handler(
                &|lhs, rhs| F::sqrt_ratio(&lhs, &rhs).1,
                &LiftInner::SqrtRatio,
            ),
            &|cond, lhs, rhs| if cond { lhs } else { rhs },
        )
    }

    fn simplified_in_arena(&self, arena: &mut MutexGuard<BumpArena>) -> Self {
        let r = match self.simplify_impl(arena, true) {
            Ok(f) => LiftInner::r#const(f),
            Err(e) => e,
        };
        arena.insert(r).into()
    }

    fn canonicalized_in_arena(&self, arena: &mut MutexGuard<BumpArena>) -> Self {
        match self {
            s @ Lift::Assigned { .. } => *s,
            Lift::Zero => lazy_init_zero::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::One => lazy_init_one::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::TwoInv => lazy_init_two_inv::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::MultiplicativeGenerator => {
                lazy_init_mult_gen::<F, Self>(arena, |_, idx| (*idx).into())
            }
            Lift::RootOfUnity => lazy_init_root::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::RootOfUnityInv => lazy_init_root_inv::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::Delta => lazy_init_delta::<F, Self>(arena, |_, idx| (*idx).into()),
            Lift::Const(f) => arena.insert(LiftInner::r#const(*f)).into(),
        }
    }

    fn unwrap_many<'a, 'b: 'a, const N: usize>(
        values: [&Self; N],
        arena: &'b mut MutexGuard<BumpArena>,
    ) -> [Unwrapped<'a, F>; N] {
        let assigned = values
            .into_iter()
            .map(|s| s.canonicalized_in_arena(arena))
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

impl<F: PrimeField + 'static> LiftLike for Lift<F> {
    type Inner = F;

    fn evaluate<T>(
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
        arena!(|arena| {
            self.evaluate_in_arena(
                arena,
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

    fn simplified(&self) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| { self.simplified_in_arena(arena) })
    }

    fn concretized(&self) -> Option<F> {
        arena!(|arena| self.simplify_impl(arena, false).ok())
    }

    fn lift() -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| { arena.insert(LiftInner::lift()).into() })
    }

    fn lift_value(f: F) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            arena.insert(LiftInner::lift_value(f)).into()
        })
    }

    fn canonicalized(&self) -> Self {
        arena!(|arena: &mut MutexGuard<BumpArena>| { self.canonicalized_in_arena(arena) })
    }
}

impl<F: PrimeField + 'static> ConstantTimeEq for Lift<F> {
    fn ct_eq(&self, other: &Self) -> Choice {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many(
                [
                    &self
                        .canonicalized_in_arena(arena)
                        .simplified_in_arena(arena),
                    &other
                        .canonicalized_in_arena(arena)
                        .simplified_in_arena(arena),
                ],
                arena,
            );
            lhs.ct_eq(&rhs)
        })
    }
}

impl<F: PrimeField> PartialEq for Lift<F> {
    fn eq(&self, other: &Self) -> bool {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            let [lhs, rhs] = Self::unwrap_many(
                [
                    &self
                        .canonicalized_in_arena(arena)
                        .simplified_in_arena(arena),
                    &other
                        .canonicalized_in_arena(arena)
                        .simplified_in_arena(arena),
                ],
                arena,
            );
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
            Some(it) => iter.fold(*it, |acc, f| acc * f),
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
            let [lhs, rhs] = Self::unwrap_many([self, rhs], arena);
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
            let [lhs, rhs] = Self::unwrap_many([self, rhs], arena);
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
            let [lhs, rhs] = Self::unwrap_many([&self, rhs], arena);
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

impl<F: fmt::Debug + PrimeField> fmt::Debug for Lift<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(v) = self.concretized() {
            return write!(f, "{v:?}");
        }
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
            Lift::Const(v) => write!(f, "{v:?}"),
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
pub struct LiftRepr([u8; 8]);

impl AsRef<[u8]> for LiftRepr {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for LiftRepr {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
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
    type Repr = F::Repr;

    /// Uses the inner representation of F and loads a lifted value that represents that it comes
    /// from off-circuit.
    fn from_repr(repr: Self::Repr) -> CtOption<Self> {
        arena!(|arena: &mut MutexGuard<BumpArena>| {
            F::from_repr(repr).map(|f| arena.insert(LiftInner::lift_value(f)).into())
        })
    }

    fn to_repr(&self) -> Self::Repr {
        self.concretized().unwrap().to_repr()
    }

    fn is_odd(&self) -> Choice {
        unimplemented!()
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

impl<F: PrimeField + Ord> Ord for Lift<F> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let cself = self.canonicalized();
        let cother = other.canonicalized();
        cself.get_index().unwrap().cmp(&cother.get_index().unwrap())
    }
}

impl<F: PrimeField + PartialOrd> PartialOrd for Lift<F> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get_index()
            .zip(other.get_index())
            .and_then(|(lhs, rhs)| lhs.partial_cmp(&rhs))
    }
}

impl<F: PrimeField + FromUniformBytes<64>> FromUniformBytes<64> for Lift<F> {
    fn from_uniform_bytes(bytes: &[u8; 64]) -> Self {
        Self::from_const(F::from_uniform_bytes(bytes))
    }
}
