use llzk_sys::{
    llzkAttributeIsAFeltConstAttr, llzkFeltConstAttrGet, llzkFeltConstAttrParseFromBase10Str,
};
use melior::{
    ir::{Attribute, AttributeLike},
    Context, StringRef,
};
use mlir_sys::MlirAttribute;

#[derive(Debug)]
pub enum Radix {
    Base2,
    Base8,
    Base10,
    Base16,
    Base32,
}

impl Default for Radix {
    fn default() -> Self {
        Self::Base10
    }
}

impl From<Radix> for u8 {
    fn from(value: Radix) -> Self {
        match value {
            Radix::Base2 => 2,
            Radix::Base8 => 8,
            Radix::Base10 => 10,
            Radix::Base16 => 16,
            Radix::Base32 => 32,
        }
    }
}

pub struct FeltConstAttribute<'c> {
    inner: Attribute<'c>,
}

impl<'c> FeltConstAttribute<'c> {
    /// # Safety
    /// The MLIR attribute must be a valid pointer of type FeltConstAttribute.
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

    pub fn parse(ctx: &'c Context, value: &str) -> Self {
        let value = StringRef::new(value);
        unsafe {
            Self::from_raw(llzkFeltConstAttrParseFromBase10Str(
                ctx.to_raw(),
                value.to_raw(),
            ))
        }
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

impl<'c> From<FeltConstAttribute<'c>> for Attribute<'c> {
    fn from(attr: FeltConstAttribute<'c>) -> Attribute<'c> {
        attr.inner
    }
}
