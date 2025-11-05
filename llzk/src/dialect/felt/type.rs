use llzk_sys::{llzkFeltTypeGet, llzkTypeIsAFeltType};
use melior::{
    Context,
    ir::{Type, TypeLike},
};
use mlir_sys::MlirType;

/// Represents the `!felt.type` type.
#[derive(Debug, Eq, PartialEq)]
pub struct FeltType<'c> {
    r#type: Type<'c>,
}

impl<'c> FeltType<'c> {
    unsafe fn from_raw(raw: MlirType) -> Self {
        Self {
            r#type: unsafe { Type::from_raw(raw) },
        }
    }

    /// Creates a new felt type.
    pub fn new(ctx: &'c Context) -> Self {
        unsafe { Self::from_raw(llzkFeltTypeGet(ctx.to_raw())) }
    }
}

impl<'c> TypeLike<'c> for FeltType<'c> {
    fn to_raw(&self) -> MlirType {
        self.r#type.to_raw()
    }
}

impl<'c> TryFrom<Type<'c>> for FeltType<'c> {
    type Error = melior::Error;

    fn try_from(t: Type<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkTypeIsAFeltType(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::TypeExpected("llzk felt", t.to_string()))
        }
    }
}

impl<'c> std::fmt::Display for FeltType<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.r#type, formatter)
    }
}

impl<'c> From<FeltType<'c>> for Type<'c> {
    fn from(t: FeltType<'c>) -> Type<'c> {
        t.r#type
    }
}
