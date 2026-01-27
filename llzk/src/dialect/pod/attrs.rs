//! Implementation of pod dialect RecordAttribute.

use llzk_sys::{
    llzkAttributeIsARecordAttr, llzkRecordAttrGet, llzkRecordAttrGetName, llzkRecordAttrGetType,
};
use melior::{
    StringRef,
    ir::{Attribute, AttributeLike, Type, TypeLike},
};
use mlir_sys::MlirAttribute;

/// A record entry within a [super::r#type::PodType].
#[derive(Clone, Copy)]
pub struct PodRecordAttribute<'c> {
    inner: Attribute<'c>,
}

impl<'c> PodRecordAttribute<'c> {
    /// # Safety
    /// The MLIR attribute must contain a valid pointer of type `RecordAttr`.
    pub unsafe fn from_raw(attr: MlirAttribute) -> Self {
        unsafe {
            Self {
                inner: Attribute::from_raw(attr),
            }
        }
    }

    /// Creates a [`PodRecordAttribute`] with the given name and type.
    pub fn new(name: &str, r#type: Type<'c>) -> Self {
        let name = StringRef::new(name);
        unsafe { Self::from_raw(llzkRecordAttrGet(name.to_raw(), r#type.to_raw())) }
    }

    /// Returns the record name.
    pub fn name(&self) -> StringRef<'c> {
        unsafe { StringRef::from_raw(llzkRecordAttrGetName(self.to_raw())) }
    }

    /// Returns the record type.
    pub fn r#type(&self) -> Type<'c> {
        unsafe { Type::from_raw(llzkRecordAttrGetType(self.to_raw())) }
    }
}

impl<'c> AttributeLike<'c> for PodRecordAttribute<'c> {
    fn to_raw(&self) -> MlirAttribute {
        self.inner.to_raw()
    }
}

impl<'c> TryFrom<Attribute<'c>> for PodRecordAttribute<'c> {
    type Error = melior::Error;

    fn try_from(t: Attribute<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkAttributeIsARecordAttr(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::AttributeExpected("llzk record", t.to_string()))
        }
    }
}

impl<'c> std::fmt::Debug for PodRecordAttribute<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PodRecordAttribute(")?;
        std::fmt::Display::fmt(&self.inner, f)?;
        write!(f, ")")
    }
}

impl<'c> std::fmt::Display for PodRecordAttribute<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'c> From<PodRecordAttribute<'c>> for Attribute<'c> {
    fn from(attr: PodRecordAttribute<'c>) -> Attribute<'c> {
        attr.inner
    }
}
