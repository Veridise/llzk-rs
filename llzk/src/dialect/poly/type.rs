//! Implementation of `!poly.tvar` type.

use crate::utils::IsA;
use llzk_sys::{llzkTypeIsATypeVarType, llzkTypeVarTypeGet, llzkTypeVarTypeGetNameRef};
use melior::{
    Context, StringRef,
    ir::{Type, TypeLike},
};
use mlir_sys::MlirType;

/// Represents the `!poly.tvar` type.
#[derive(Debug, Eq, PartialEq)]
pub struct TVarType<'c> {
    r#type: Type<'c>,
}

impl<'c> TVarType<'c> {
    unsafe fn from_raw(raw: MlirType) -> Self {
        Self {
            r#type: unsafe { Type::from_raw(raw) },
        }
    }

    /// Creates a new `!poly.tvar` type variable with the given symbol name.
    pub fn new(ctx: &'c Context, name: StringRef) -> Self {
        unsafe { Self::from_raw(llzkTypeVarTypeGet(ctx.to_raw(), name.to_raw())) }
    }

    /// Returns the name of the type variable.
    pub fn name(&self) -> StringRef<'_> {
        unsafe { StringRef::from_raw(llzkTypeVarTypeGetNameRef(self.r#type.to_raw())) }
    }
}

impl<'c> TypeLike<'c> for TVarType<'c> {
    fn to_raw(&self) -> MlirType {
        self.r#type.to_raw()
    }
}

impl<'c> TryFrom<Type<'c>> for TVarType<'c> {
    type Error = melior::Error;

    fn try_from(t: Type<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkTypeIsATypeVarType(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::TypeExpected(
                "llzk type variable",
                t.to_string(),
            ))
        }
    }
}

impl<'c> std::fmt::Display for TVarType<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.r#type, formatter)
    }
}

impl<'c> From<TVarType<'c>> for Type<'c> {
    fn from(t: TVarType<'c>) -> Type<'c> {
        t.r#type
    }
}

/// Return `true` iff the given [Type] is a [TVarType].
#[inline]
pub fn is_type_variable(t: Type) -> bool {
    t.isa::<TVarType>()
}
