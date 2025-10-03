use llzk_sys::{
    llzkAttributeIsAFeltConstAttr, llzkFeltConstAttrGet, llzkFeltConstAttrGetFromParts,
    llzkFeltConstAttrGetFromString,
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

    /// Creates a [`FeltConstAttribute`] from a base 10 string representation.
    pub fn parse(ctx: &'c Context, bitlen: u32, value: &str) -> Self {
        let value = StringRef::new(value);
        unsafe {
            Self::from_raw(llzkFeltConstAttrGetFromString(
                ctx.to_raw(),
                bitlen,
                value.to_raw(),
            ))
        }
    }

    /// Creates a [`FeltConstAttribute`] from a slice of bigint parts in LSB order.
    ///
    /// If the number represented by the parts is unsigned set the bit length to at least one more
    /// than the minimum number of bits required to represent it. Otherwise the number will be
    /// interpreted as signed and may cause unexpected behaviors.
    pub fn from_parts(ctx: &'c Context, bitlen: u32, parts: &[u64]) -> Self {
        unsafe {
            Self::from_raw(llzkFeltConstAttrGetFromParts(
                ctx.to_raw(),
                bitlen,
                parts.as_ptr(),
                parts.len() as isize,
            ))
        }
    }

    /// Creates a [`FeltConstAttribute`] from a [`num_bigint::BigUint`].
    ///
    /// Panics if the number of bits required to represent the bigint plus one does not fit in 32 bits.
    #[cfg(feature = "bigint")]
    pub fn from_biguint(ctx: &'c Context, value: &num_bigint::BigUint) -> Self {
        // Increase by one to ensure the value is kept unsigned.
        let bitlen = value.bits() + 1;
        let parts = value.to_u64_digits();
        Self::from_parts(ctx, bitlen.try_into().unwrap(), &parts)
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

impl<'c> std::fmt::Debug for FeltConstAttribute<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "FeltConstAttribute(")?;
        std::fmt::Display::fmt(&self.inner, f)?;
        write!(f, ")")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn felt_const_attr_new(value: u64) {
        let ctx = LlzkContext::new();
        let _ = FeltConstAttribute::new(&ctx, value);
    }

    #[quickcheck]
    fn felt_const_attr_parse_from_u64(value: u64) {
        let ctx = LlzkContext::new();
        let _ = FeltConstAttribute::parse(&ctx, 64, &value.to_string());
    }
}
