use llzk_sys::llzkTypeIsAStructType;
use melior::ir::{Type, TypeLike};
use mlir_sys::MlirType;

use crate::utils::FromRaw;

pub struct StructType<'c> {
    t: Type<'c>,
}

impl<'c> FromRaw<MlirType> for StructType<'c> {
    unsafe fn from_raw(t: MlirType) -> Self {
        Self {
            t: unsafe { Type::from_raw(t) },
        }
    }
}

impl<'c> TypeLike<'c> for StructType<'c> {
    fn to_raw(&self) -> MlirType {
        self.t.to_raw()
    }
}

impl<'c> TryFrom<Type<'c>> for StructType<'c> {
    type Error = melior::Error;

    fn try_from(t: Type<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkTypeIsAStructType(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::TypeExpected("llzk struct", t.to_string()))
        }
    }
}

impl<'c> std::fmt::Display for StructType<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.t, formatter)
    }
}

impl<'c> From<StructType<'c>> for Type<'c> {
    fn from(s: StructType<'c>) -> Type<'c> {
        s.t
    }
}
