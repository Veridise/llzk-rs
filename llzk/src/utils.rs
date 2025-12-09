//! General utilities

use melior::{
    StringRef,
    ir::{
        Block, BlockRef, Operation, OperationRef, Region, RegionLike, RegionRef,
        operation::OperationLike,
    },
};
use mlir_sys::MlirStringRef;
use std::{
    ffi::c_void,
    fmt::{self, Formatter},
};

/// Creates an instance from its low-level unsafe representation.
pub trait FromRaw<RawT> {
    /// Constructs Self from RawT via some unsafe function.
    /// # Safety
    /// The raw value must be a valid reference to some MLIR object.
    unsafe fn from_raw(raw: RawT) -> Self;
}

#[allow(dead_code)]
pub(crate) unsafe extern "C" fn print_callback(string: MlirStringRef, data: *mut c_void) {
    unsafe {
        let (formatter, result) = &mut *(data as *mut (&mut Formatter, fmt::Result));

        if result.is_err() {
            return;
        }

        *result = (|| {
            write!(
                formatter,
                "{}",
                StringRef::from_raw(string)
                    .as_str()
                    .map_err(|_| fmt::Error)?
            )
        })();
    }
}

/// Creates an [`Identifier`].
///
/// [`Identifier`]: [`melior::ir::Identifier`].
#[macro_export]
macro_rules! ident {
    ($ctx:expr, $name:expr) => {{
        let ctx = $ctx;
        melior::ir::Identifier::new(unsafe { ctx.to_ref() }, $name)
    }};
}

/// Trait for converting melior types to their reference counterparts.
///
/// This trait provides a safe interface for types that have a `to_raw()` and `from_raw()` pattern,
/// enabling conversion from owned types to reference types (e.g., `Block` to `BlockRef`).
pub trait IntoRef<RefType> {
    /// Convert this type into its reference counterpart.
    fn into_ref(self) -> RefType;
}

/// Macro to implement `IntoRef` for melior types with the `to_raw()` + `from_raw()` pattern.
macro_rules! impl_into_ref {
    ($owned:ty, $ref:ty) => {
        impl<'c, 'a> IntoRef<$ref> for $owned {
            #[inline]
            fn into_ref(self) -> $ref {
                unsafe { <$ref>::from_raw(self.to_raw()) }
            }
        }
    };
}

impl_into_ref!(Block<'c>, BlockRef<'c, 'a>);
impl_into_ref!(Region<'c>, RegionRef<'c, 'a>);
impl_into_ref!(Operation<'c>, OperationRef<'c, 'a>);
