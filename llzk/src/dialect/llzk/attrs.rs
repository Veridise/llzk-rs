use crate::attributes::NamedAttribute;
use llzk_sys::{
    llzkAttributeIsALoopBoundsAttr, llzkAttributeIsAPublicAttr, llzkLoopBoundsAttrGet,
    llzkPublicAttrGet,
};
use melior::{
    Context,
    ir::{Attribute, AttributeLike, Identifier},
};
use mlir_sys::MlirAttribute;

/// Represents the `llzk.pub` attribute.
#[derive(Debug)]
pub struct PublicAttribute<'c> {
    inner: Attribute<'c>,
}

impl<'c> PublicAttribute<'c> {
    /// Creates a new attribute from its raw representation.
    ///
    /// # Safety
    ///
    /// The MLIR attribute must contain a valid pointer of type `PublicAttr`.
    pub unsafe fn from_raw(attr: MlirAttribute) -> Self {
        unsafe {
            Self {
                inner: Attribute::from_raw(attr),
            }
        }
    }

    /// Creates a new attribute.
    pub fn new(ctx: &'c Context) -> Self {
        unsafe { Self::from_raw(llzkPublicAttrGet(ctx.to_raw())) }
    }

    /// Creates a new `llzk.pub` NamedAttribute.
    pub fn new_named_attr(ctx: &'c Context) -> NamedAttribute<'c> {
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

/// Represents the `llzk.loopbounds` attribute.
#[derive(Debug)]
pub struct LoopBoundsAttribute<'c> {
    inner: Attribute<'c>,
}

impl<'c> LoopBoundsAttribute<'c> {
    /// Creates a new attribute from its raw representation.
    ///
    /// # Safety
    ///
    /// The MLIR attribute must contain a valid pointer of type `LoopBoundsAttr`.
    pub unsafe fn from_raw(attr: MlirAttribute) -> Self {
        unsafe {
            Self {
                inner: Attribute::from_raw(attr),
            }
        }
    }

    /// Creates a new attribute.
    pub fn new(ctx: &'c Context, begin: i64, end: i64, step: i64) -> Self {
        unsafe { Self::from_raw(llzkLoopBoundsAttrGet(ctx.to_raw(), begin, end, step)) }
    }
}

impl<'c> AttributeLike<'c> for LoopBoundsAttribute<'c> {
    fn to_raw(&self) -> MlirAttribute {
        self.inner.to_raw()
    }
}

impl<'c> TryFrom<Attribute<'c>> for LoopBoundsAttribute<'c> {
    type Error = melior::Error;

    fn try_from(t: Attribute<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkAttributeIsALoopBoundsAttr(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::AttributeExpected(
                "llzk loopbounds",
                t.to_string(),
            ))
        }
    }
}

impl<'c> std::fmt::Display for LoopBoundsAttribute<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'c> From<LoopBoundsAttribute<'c>> for Attribute<'c> {
    fn from(attr: LoopBoundsAttribute<'c>) -> Attribute<'c> {
        attr.inner
    }
}
