use std::{fmt, ops::AddAssign};

#[cfg(feature = "bigint-felt")]
use num_bigint::BigUint;

use crate::display::{TextRepresentable, TextRepresentation};

pub trait IntoPrime: Into<Felt> {
    fn prime() -> Felt;
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg(feature = "bigint-felt")]
pub struct Felt(BigUint);
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg(not(feature = "bigint-felt"))]
pub struct Felt(usize);

#[cfg(feature = "bigint-felt")]
impl Felt {
    pub fn new(v: BigUint) -> Self {
        Self(v)
    }
}

#[cfg(not(feature = "bigint-felt"))]
impl Felt {
    pub fn new(v: usize) -> Self {
        Self(v)
    }
}

impl Felt {
    pub fn prime<I: IntoPrime>() -> Felt {
        I::prime()
    }

    pub fn is_one(&self) -> bool {
        self.0 == 1usize.into()
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0usize.into()
    }
}

impl TextRepresentable for Felt {
    fn to_repr(&self) -> TextRepresentation {
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

#[cfg(not(feature = "bigint-felt"))]
#[cfg(test)]
pub mod tests {
    use super::{Felt, IntoPrime};

    pub struct Seven(u8);

    impl Into<Felt> for Seven {
        fn into(self) -> Felt {
            Felt(self.0.into())
        }
    }

    impl IntoPrime for Seven {
        fn prime() -> Felt {
            Felt(7)
        }
    }

    const ZERO: Felt = Felt(0);
    const ONE: Felt = Felt(1);

    #[test]
    fn is_zero() {
        assert!(!ONE.is_zero());
        assert!(ZERO.is_zero());
    }

    #[test]
    fn is_one() {
        assert!(ONE.is_one());
        assert!(!ZERO.is_one());
    }

    #[test]
    fn prime() {
        assert_eq!(Felt::prime::<Seven>(), Felt(7))
    }
}

#[cfg(feature = "bigint-felt")]
#[cfg(test)]
pub mod tests {
    use super::{Felt, IntoPrime};

    pub struct Seven(u8);

    impl Into<Felt> for Seven {
        fn into(self) -> Felt {
            Felt(self.0.into())
        }
    }

    impl IntoPrime for Seven {
        fn prime() -> Felt {
            Felt(7usize.into())
        }
    }

    #[test]
    fn prime() {
        assert_eq!(Felt::prime::<Seven>(), Felt(7usize.into()))
    }
}
