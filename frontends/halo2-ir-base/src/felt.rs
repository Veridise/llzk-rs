//! Felt type.

use ff::PrimeField;
use internment::Intern;
use num_bigint::BigUint;
use std::ops::{Add, Deref, Mul, Rem, RemAssign, Sub};

/// Lightweight representation of a constant value.
///
/// The actual value is interned which allows this type to be [`Copy`] and
/// avoids duplicatation of commonly used values.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Felt(Intern<BigUint>);

impl Felt {
    /// Creates a new felt from an implementation of [`PrimeField`].
    pub fn new<F: PrimeField>(f: F) -> Self {
        Self(Intern::new(BigUint::from_bytes_le(f.to_repr().as_ref())))
    }

    /// Creates a new felt whose value is the prime in the [`PrimeField`].
    pub fn prime<F: PrimeField>() -> Self {
        let f = -F::ONE;
        Self(Intern::new(
            BigUint::from_bytes_le(f.to_repr().as_ref()) + 1usize,
        ))
    }

    /// Creates a felt from anything that can become a [`BigUint`].
    ///
    /// Use this method only during testing.
    #[cfg(test)]
    pub fn new_from<I: Into<BigUint>>(i: I) -> Self {
        Self(Intern::new(i.into()))
    }
}

impl std::fmt::Debug for Felt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

//#[cfg(feature = "picus-backend")]
//impl From<Felt> for picus::felt::Felt {
//    fn from(value: Felt) -> Self {
//        Self::new(value.0.as_ref().clone())
//    }
//}

impl<T: Into<BigUint>> From<T> for Felt {
    fn from(value: T) -> Self {
        Self(Intern::new(value.into()))
    }
}

impl<T> PartialEq<T> for Felt
where
    T: Into<BigUint> + Copy,
{
    fn eq(&self, other: &T) -> bool {
        self.as_ref().eq(&(*other).into())
    }
}

impl AsRef<BigUint> for Felt {
    fn as_ref(&self) -> &BigUint {
        self.0.as_ref()
    }
}

impl Deref for Felt {
    type Target = BigUint;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Rem for Felt {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        if self < rhs {
            return self;
        }
        ((*self).clone() % (*rhs).clone()).into()
    }
}

impl RemAssign for Felt {
    fn rem_assign(&mut self, rhs: Self) {
        if *self > rhs {
            *self = *self % rhs;
        }
    }
}

impl Sub for Felt {
    type Output = Option<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self < rhs {
            return None;
        }

        Some(((*self).clone() - (*rhs).clone()).into())
    }
}

impl Add for Felt {
    type Output = Felt;

    fn add(self, rhs: Self) -> Self::Output {
        ((*self).clone() + (*rhs).clone()).into()
    }
}

impl Mul for Felt {
    type Output = Felt;

    fn mul(self, rhs: Self) -> Self::Output {
        ((*self).clone() * (*rhs).clone()).into()
    }
}

impl std::fmt::Display for Felt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}
