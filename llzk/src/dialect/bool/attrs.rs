use llzk_sys::{LlzkCmp, llzkAttributeIsAFeltCmpPredicateAttr, llzkFeltCmpPredicateAttrGet};
use melior::{
    Context,
    ir::{Attribute, AttributeLike},
};
use mlir_sys::MlirAttribute;

/// Possible options for creating [`CmpPredicateAttribute`].
#[derive(Debug)]
#[repr(u32)]
pub enum CmpPredicate {
    /// Equal to.
    Eq = llzk_sys::LlzkCmp_LlzkCmp_EQ,
    /// Not equal to.
    Ne = llzk_sys::LlzkCmp_LlzkCmp_NE,
    /// Less than.
    Lt = llzk_sys::LlzkCmp_LlzkCmp_LT,
    /// Less than or equal to.
    Le = llzk_sys::LlzkCmp_LlzkCmp_LE,
    /// Greater than.
    Gt = llzk_sys::LlzkCmp_LlzkCmp_GT,
    /// Greater than or equal to.
    Ge = llzk_sys::LlzkCmp_LlzkCmp_GE,
}

/// Attribute representing a comparison predicate.
#[derive(Clone, Copy, Debug)]
pub struct CmpPredicateAttribute<'c> {
    inner: Attribute<'c>,
}

impl<'c> CmpPredicateAttribute<'c> {
    /// Creates a new attribute from its raw representation.
    ///
    /// # Safety
    ///
    /// The MLIR attribute must contain a valid pointer of type `CmpPredicateAttr`.
    pub unsafe fn from_raw(attr: MlirAttribute) -> Self {
        unsafe {
            Self {
                inner: Attribute::from_raw(attr),
            }
        }
    }

    /// Creates a new attribute.
    pub fn new(ctx: &'c Context, predicate: CmpPredicate) -> Self {
        unsafe {
            Self::from_raw(llzkFeltCmpPredicateAttrGet(
                ctx.to_raw(),
                predicate as LlzkCmp,
            ))
        }
    }
}

impl<'c> AttributeLike<'c> for CmpPredicateAttribute<'c> {
    fn to_raw(&self) -> MlirAttribute {
        self.inner.to_raw()
    }
}

impl<'c> TryFrom<Attribute<'c>> for CmpPredicateAttribute<'c> {
    type Error = melior::Error;

    fn try_from(t: Attribute<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkAttributeIsAFeltCmpPredicateAttr(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::AttributeExpected(
                "llzk cmp attr",
                t.to_string(),
            ))
        }
    }
}

impl<'c> std::fmt::Display for CmpPredicateAttribute<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'c> From<CmpPredicateAttribute<'c>> for Attribute<'c> {
    fn from(attr: CmpPredicateAttribute<'c>) -> Attribute<'c> {
        attr.inner
    }
}
