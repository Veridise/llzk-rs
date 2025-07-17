use std::fmt;

use melior::{
    ir::{attribute::FlatSymbolRefAttribute, Attribute, AttributeLike},
    Context, StringRef,
};
use mlir_sys::{
    mlirAttributeIsASymbolRef, mlirSymbolRefAttrGet, mlirSymbolRefAttrGetLeafReference,
    mlirSymbolRefAttrGetNestedReference, mlirSymbolRefAttrGetNumNestedReferences,
    mlirSymbolRefAttrGetRootReference, MlirAttribute,
};

#[derive(Clone, Copy)]
pub struct SymbolRefAttribute<'c> {
    inner: Attribute<'c>,
}

impl<'c> SymbolRefAttribute<'c> {
    pub fn new(ctx: &'c Context, name: &str, refs: &[&str]) -> Self {
        let name = StringRef::new(name);
        let refs: Vec<_> = refs
            .iter()
            .map(|r| FlatSymbolRefAttribute::new(ctx, r))
            .collect();
        let raw_refs: Vec<_> = refs.iter().map(|r| r.to_raw()).collect();

        Self {
            inner: unsafe {
                Attribute::from_raw(mlirSymbolRefAttrGet(
                    ctx.to_raw(),
                    name.to_raw(),
                    raw_refs.len() as isize,
                    raw_refs.as_ptr(),
                ))
            },
        }
    }

    pub fn root(&self) -> StringRef {
        unsafe { StringRef::from_raw(mlirSymbolRefAttrGetRootReference(self.to_raw())) }
    }

    pub fn leaf(&self) -> StringRef {
        unsafe { StringRef::from_raw(mlirSymbolRefAttrGetLeafReference(self.to_raw())) }
    }

    pub fn nested(&self) -> Vec<Attribute<'c>> {
        let nested_count = unsafe { mlirSymbolRefAttrGetNumNestedReferences(self.to_raw()) };
        (0..nested_count)
            .map(|i| unsafe {
                Attribute::from_raw(mlirSymbolRefAttrGetNestedReference(self.to_raw(), i))
            })
            .collect()
    }
}

impl<'c> AttributeLike<'c> for SymbolRefAttribute<'c> {
    fn to_raw(&self) -> MlirAttribute {
        self.inner.to_raw()
    }
}

impl<'c> TryFrom<Attribute<'c>> for SymbolRefAttribute<'c> {
    type Error = melior::Error;

    fn try_from(value: Attribute<'c>) -> Result<Self, Self::Error> {
        if unsafe { mlirAttributeIsASymbolRef(value.to_raw()) } {
            Ok(Self { inner: value })
        } else {
            Err(Self::Error::AttributeExpected(
                "symbol ref attr",
                value.to_string(),
            ))
        }
    }
}

impl<'c> From<SymbolRefAttribute<'c>> for Attribute<'c> {
    fn from(sym: SymbolRefAttribute<'c>) -> Attribute<'c> {
        sym.inner
    }
}

impl<'c> fmt::Display for SymbolRefAttribute<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}
