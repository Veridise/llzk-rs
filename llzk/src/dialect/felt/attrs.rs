use llzk_sys::{llzkAttributeIsAFeltConstAttr, llzkFeltConstAttrGet};
use melior::{
    ir::{Attribute, AttributeLike},
    Context,
};
use mlir_sys::MlirAttribute;

pub struct FeltConstAttribute<'c> {
    inner: Attribute<'c>,
}

impl<'c> FeltConstAttribute<'c> {
    pub unsafe fn from_raw(attr: MlirAttribute) -> Self {
        unsafe {
            Self {
                inner: Attribute::from_raw(attr),
            }
        }
    }

    pub fn new(ctx: &'c Context, value: u64) -> Self {
        unsafe { Self::from_raw(llzkFeltConstAttrGet(ctx.to_raw(), value as i64)) }
    }
}

impl<'c> AttributeLike<'c> for FeltConstAttribute<'c> {
    fn to_raw(&self) -> MlirAttribute {
        self.inner.to_raw()
    }
}

impl<'c> TryFrom<Attribute<'c>> for FeltConstAttribute<'c> {
    type Error = melior::Error;

    fn try_from(t: Attribute<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkAttributeIsAFeltConstAttr(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::AttributeExpected("llzk felt", t.to_string()))
        }
    }
}

impl<'c> std::fmt::Display for FeltConstAttribute<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'c> Into<Attribute<'c>> for FeltConstAttribute<'c> {
    fn into(self) -> Attribute<'c> {
        self.inner
    }
}
