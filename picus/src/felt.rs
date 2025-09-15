use std::{
    fmt,
    ops::{AddAssign, Rem, Sub},
};

#[cfg(feature = "bigint-felt")]
use num_bigint::BigUint;

use crate::display::{TextRepresentable, TextRepresentation};

//pub trait IntoPrime: Into<Felt> {
//    fn prime() -> Felt;
//}
#[cfg(feature = "bigint-felt")]
pub type FeltRepr = BigUint;
#[cfg(not(feature = "bigint-felt"))]
pub type FeltRepr = usize;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Felt(FeltRepr);

impl Felt {
    pub fn new(v: FeltRepr) -> Self {
        Self(v)
    }
}

#[cfg(feature = "bigint-felt")]
impl From<usize> for Felt {
    fn from(value: usize) -> Self {
        Self(value.into())
    }
}

impl From<FeltRepr> for Felt {
    fn from(value: FeltRepr) -> Self {
        Self(value)
    }
}

impl Felt {
    //pub fn prime<I: IntoPrime>() -> Felt {
    //    I::prime()
    //}

    pub fn is_one(&self) -> bool {
        self.0 == 1usize.into()
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0usize.into()
    }
}

impl TextRepresentable for Felt {
    fn to_repr(&self) -> TextRepresentation<'_> {
        TextRepresentation::owned_atom(self.to_string())
    }

    fn width_hint(&self) -> usize {
        self.to_string().len()
    }
}

impl fmt::Display for Felt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AddAssign<usize> for Felt {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub for Felt {
    type Output = Felt;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Rem for Felt {
    type Output = Felt;

    fn rem(self, rhs: Self) -> Self::Output {
        Self(self.0 % rhs.0)
    }
}

impl<R: Into<FeltRepr>> Sub<R> for Felt {
    type Output = Felt;

    fn sub(self, rhs: R) -> Self::Output {
        Self(self.0 - rhs.into())
    }
}
