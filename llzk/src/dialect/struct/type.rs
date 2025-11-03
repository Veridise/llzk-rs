use llzk_sys::{llzkStructTypeGetWithArrayAttr, llzkTypeIsAStructType};
use melior::{
    Context,
    ir::{
        Attribute, AttributeLike as _, Type, TypeLike,
        attribute::{ArrayAttribute, FlatSymbolRefAttribute},
    },
};
use mlir_sys::MlirType;

use crate::utils::FromRaw;

/// Represents the `!struct.type` type.
#[derive(Copy, Clone, Debug)]
pub struct StructType<'c> {
    t: Type<'c>,
}

impl<'c> StructType<'c> {
    /// Creates a new struct type.
    ///
    /// The params array must match the number of params and their kind as defined by the associated `struct.def`
    /// operation.
    pub fn new(name: FlatSymbolRefAttribute<'c>, params: &[Attribute<'c>]) -> Self {
        unsafe {
            Self::from_raw(llzkStructTypeGetWithArrayAttr(
                name.to_raw(),
                ArrayAttribute::new(name.context().to_ref(), params).to_raw(),
            ))
        }
    }

    /// Creates a new struct type from a string reference.
    ///
    /// The returned type won't have any parameters.
    pub fn from_str(context: &'c Context, name: &str) -> Self {
        Self::new(FlatSymbolRefAttribute::new(context, name), &[])
    }

    /// Creates a new struct type from string references for both its name and parameters.
    pub fn from_str_params(context: &'c Context, name: &str, params: &[&str]) -> Self {
        let params: Vec<Attribute> = params
            .iter()
            .map(|param| FlatSymbolRefAttribute::new(context, param).into())
            .collect();
        Self::new(FlatSymbolRefAttribute::new(context, name), &params)
    }
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
