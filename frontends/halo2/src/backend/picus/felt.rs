use std::ops::Deref;

use num_bigint::BigUint;
use picus::felt::Felt;

use crate::halo2::PrimeField;
#[cfg(feature = "lift-field-operations")]
use crate::ir::lift::LiftLike;

#[derive(Default)]
pub struct FeltWrap<F: PrimeField>(F);

impl<F: PrimeField> Deref for FeltWrap<F> {
    type Target = F;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<F: PrimeField> From<F> for FeltWrap<F> {
    fn from(value: F) -> Self {
        Self(value)
    }
}

impl<F: PrimeField> From<&F> for FeltWrap<F> {
    fn from(value: &F) -> Self {
        Self(*value)
    }
}

impl<F: PrimeField> From<FeltWrap<F>> for Felt {
    fn from(wrap: FeltWrap<F>) -> Felt {
        let r = wrap.0.to_repr();
        Felt::new(BigUint::from_bytes_le(r.as_ref()))
    }
}

//impl<F: PrimeField> IntoPrime for FeltWrap<F> {
//    fn prime() -> Felt {
//        let mut f = FeltWrap(-F::ONE).into();
//        f += 1;
//        f
//    }
//}
