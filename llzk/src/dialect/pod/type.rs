//! Implementation of `!pod.type` type.

use super::attrs::PodRecordAttribute;
use crate::utils::IsA;
use llzk_sys::{
    llzkPodTypeGet, llzkPodTypeGetNumRecords, llzkPodTypeGetRecords, llzkPodTypeLookupRecord,
    llzkTypeIsAPodType,
};
use melior::{
    Context, StringRef,
    ir::{Attribute, AttributeLike, Type, TypeLike},
};
use mlir_sys::{MlirAttribute, MlirType};

/// Represents the `!pod.type` type.
#[derive(Debug, Eq, PartialEq)]
pub struct PodType<'c> {
    r#type: Type<'c>,
}

impl<'c> PodType<'c> {
    unsafe fn from_raw(raw: MlirType) -> Self {
        Self {
            r#type: unsafe { Type::from_raw(raw) },
        }
    }

    /// Creates a new type with the given records.
    pub fn new(ctx: &'c Context, records: &[PodRecordAttribute<'c>]) -> Self {
        let raw_refs: Vec<_> = records.iter().map(|r| r.to_raw()).collect();
        unsafe {
            Self::from_raw(llzkPodTypeGet(
                ctx.to_raw(),
                raw_refs.len() as isize,
                raw_refs.as_ptr(),
            ))
        }
    }

    /// Get the list of `PodRecordAttribute` that make up this pod type.  
    ///
    /// # Panics
    ///
    /// If any of the wrapped attributes is not a `pod.record` attribute.
    pub fn get_records(&self) -> Vec<PodRecordAttribute<'c>> {
        let num = unsafe { llzkPodTypeGetNumRecords(self.to_raw()) };
        let mut raw = vec![
            MlirAttribute {
                ptr: std::ptr::null()
            };
            num.try_into().unwrap()
        ];
        unsafe { llzkPodTypeGetRecords(self.to_raw(), raw.as_mut_ptr()) };
        raw.into_iter()
            .map(|op| {
                unsafe { Attribute::from_raw(op) }
                    .try_into()
                    .expect("op of type 'pod.record'")
            })
            .collect()
    }

    /// Get the type of the record with the given name, if it exists in this type.
    pub fn get_type_of_record(&self, name: &str) -> Option<Type<'c>> {
        let name = StringRef::new(name);
        let raw = unsafe { llzkPodTypeLookupRecord(self.to_raw(), name.to_raw()) };
        (!raw.ptr.is_null()).then(|| unsafe { Type::from_raw(raw) })
    }
}

impl<'c> TypeLike<'c> for PodType<'c> {
    fn to_raw(&self) -> MlirType {
        self.r#type.to_raw()
    }
}

impl<'c> TryFrom<Type<'c>> for PodType<'c> {
    type Error = melior::Error;

    fn try_from(t: Type<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkTypeIsAPodType(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::TypeExpected("llzk pod", t.to_string()))
        }
    }
}

impl<'c> std::fmt::Display for PodType<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.r#type, formatter)
    }
}

impl<'c> From<PodType<'c>> for Type<'c> {
    fn from(t: PodType<'c>) -> Type<'c> {
        t.r#type
    }
}

/// Return `true` iff the given [Type] is an [PodType].
#[inline]
pub fn is_pod_type(t: Type) -> bool {
    t.isa::<PodType>()
}
