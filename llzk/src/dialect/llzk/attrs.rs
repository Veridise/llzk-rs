use llzk_sys::{llzkAttributeIsAPublicAttr, llzkPublicAttrGet};
use melior::{
    Context,
    ir::{Attribute, AttributeLike, Identifier},
};
use mlir_sys::MlirAttribute;

pub struct PublicAttribute<'c> {
    inner: Attribute<'c>,
}

impl<'c> PublicAttribute<'c> {
    /// # Safety
    /// The MLIR attribute must be a valid pointer of type FeltConstAttribute.
    pub unsafe fn from_raw(attr: MlirAttribute) -> Self {
        unsafe {
            Self {
                inner: Attribute::from_raw(attr),
            }
        }
    }

    pub fn new(ctx: &'c Context) -> Self {
        unsafe { Self::from_raw(llzkPublicAttrGet(ctx.to_raw())) }
    }

    pub fn named_attr_pair(ctx: &'c Context) -> (Identifier<'c>, Attribute<'c>) {
        (Identifier::new(ctx, "llzk.pub"), Attribute::unit(ctx))
    }
}

impl<'c> AttributeLike<'c> for PublicAttribute<'c> {
    fn to_raw(&self) -> MlirAttribute {
        self.inner.to_raw()
    }
}

impl<'c> TryFrom<Attribute<'c>> for PublicAttribute<'c> {
    type Error = melior::Error;

    fn try_from(t: Attribute<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkAttributeIsAPublicAttr(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::AttributeExpected("llzk pub", t.to_string()))
        }
    }
}

impl<'c> std::fmt::Display for PublicAttribute<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'c> From<PublicAttribute<'c>> for Attribute<'c> {
    fn from(attr: PublicAttribute<'c>) -> Attribute<'c> {
        attr.inner
    }
}
