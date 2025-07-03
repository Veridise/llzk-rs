use llzk_sys::{
    llzkArrayTypeGet, llzkArrayTypeGetDim, llzkArrayTypeGetElementType, llzkArrayTypeGetNumDims,
    llzkArrayTypeGetWithNumericDims, llzkTypeIsAArrayType,
};
use melior::ir::{Attribute, Type, TypeLike};
use mlir_sys::MlirType;

#[derive(Debug, Eq, PartialEq)]
pub struct ArrayType<'c> {
    r#type: Type<'c>,
}

impl<'c> ArrayType<'c> {
    unsafe fn from_raw(raw: MlirType) -> Self {
        Self {
            r#type: unsafe { Type::from_raw(raw) },
        }
    }

    pub fn new(element_type: Type<'c>, dims: &[Attribute<'c>]) -> Self {
        unsafe {
            Self::from_raw(llzkArrayTypeGet(
                element_type.to_raw(),
                dims.len() as _,
                dims.as_ptr() as *const _,
            ))
        }
    }

    pub fn new_with_dims(element_type: Type<'c>, dims: &[i64]) -> Self {
        unsafe {
            Self::from_raw(llzkArrayTypeGetWithNumericDims(
                element_type.to_raw(),
                dims.len() as _,
                dims.as_ptr() as *const _,
            ))
        }
    }

    pub fn element_type(&self) -> Type<'c> {
        unsafe { Type::from_raw(llzkArrayTypeGetElementType(self.to_raw())) }
    }

    pub fn num_dims(&self) -> isize {
        unsafe { llzkArrayTypeGetNumDims(self.to_raw()) }
    }

    pub fn dim(&self, idx: isize) -> Attribute<'c> {
        unsafe { Attribute::from_raw(llzkArrayTypeGetDim(self.to_raw(), idx)) }
    }
}

impl<'c> TypeLike<'c> for ArrayType<'c> {
    fn to_raw(&self) -> MlirType {
        self.r#type.to_raw()
    }
}

impl<'c> TryFrom<Type<'c>> for ArrayType<'c> {
    type Error = melior::Error;

    fn try_from(t: Type<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkTypeIsAArrayType(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::TypeExpected("llzk array", t.to_string()))
        }
    }
}

impl<'c> std::fmt::Display for ArrayType<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.r#type, formatter)
    }
}

impl<'c> From<ArrayType<'c>> for Type<'c> {
    fn from(t: ArrayType<'c>) -> Type<'c> {
        t.r#type
    }
}
