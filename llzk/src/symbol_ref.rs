//! Types related to the SymbolRef attribute.

use std::fmt;

use melior::{
    Context, StringRef,
    ir::{Attribute, AttributeLike, attribute::FlatSymbolRefAttribute},
};
use mlir_sys::{
    MlirAttribute, mlirAttributeIsASymbolRef, mlirSymbolRefAttrGet,
    mlirSymbolRefAttrGetLeafReference, mlirSymbolRefAttrGetNestedReference,
    mlirSymbolRefAttrGetNumNestedReferences, mlirSymbolRefAttrGetRootReference,
};

/// A `SymbolRef` attribute.
///
/// The difference between this attribute and [`FlatSymbolRefAttribute`] is that this attribute
/// contains a path of symbol names, which allows referencing operations defined inside modules,
/// structs, etc.
#[derive(Clone, Copy, Debug)]
pub struct SymbolRefAttribute<'c> {
    inner: Attribute<'c>,
}

impl<'c> SymbolRefAttribute<'c> {
    /// Creates a new symbol attribute with the give name and path.
    ///
    /// The path doesn't contain the name of the symbol. Therefore, for symbols defined at the _top
    /// level_ the `refs` slice should be empty.
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

    /// Returns the root of the symbol's path.
    pub fn root(&self) -> StringRef<'_> {
        unsafe { StringRef::from_raw(mlirSymbolRefAttrGetRootReference(self.to_raw())) }
    }

    /// Returns the leaf of the symbol's path. This corresponds with the symbol name.
    pub fn leaf(&self) -> StringRef<'_> {
        unsafe { StringRef::from_raw(mlirSymbolRefAttrGetLeafReference(self.to_raw())) }
    }

    /// Returns the symbol path as a vector of independent attributes.
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
