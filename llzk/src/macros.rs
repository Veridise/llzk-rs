macro_rules! concrete_op_type {
    ($type:ident, $isa:ident, $opname:literal) => {
        crate::macros::concrete_op_type!(@def $type, concat!("Represents an owned '", $opname, "' op."));
        crate::macros::concrete_op_type!(@impl $type,
            concat!("The MLIR operation must be a valid pointer of type ", stringify!($type) ,"."));
        crate::macros::concrete_op_type!(@op_like $type);
        crate::macros::concrete_op_type!(@display $type);
        crate::macros::concrete_op_type!(@into $type);
        crate::macros::concrete_op_type!(@try_from $type, $isa, $opname);
    };
    (@def $type:ident,  $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug)]
        pub struct $type<'c> {
            raw: mlir_sys::MlirOperation,
            _context: std::marker::PhantomData<&'c melior::Context>,
        }
    };
    (@impl $type:ident,  $doc:expr) => {
        impl<'c> $type<'c> {
            /// # Safety
            #[doc = $doc]
            pub unsafe fn from_raw(raw: mlir_sys::MlirOperation) -> Self {
                Self {
                    raw,
                    _context: std::marker::PhantomData,
                }
            }

            /// Converts an operation into a raw object.
            pub const fn into_raw(self) -> mlir_sys::MlirOperation {
                let operation = self.raw;

                core::mem::forget(self);

                operation
            }
        }
    };
    (@op_like $type:ident) => {
        impl<'a, 'c: 'a> melior::ir::operation::OperationLike<'c, 'a> for $type<'c> {
            fn to_raw(&self) -> mlir_sys::MlirOperation {
                self.raw
            }
        }
    };
    (@display $type:ident) => {
        impl std::fmt::Display for $type<'_> {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                let r = unsafe { melior::ir::operation::OperationRef::from_raw(self.raw) };
                std::fmt::Display::fmt(&r, formatter)
            }
        }
    };
    (@into $type:ident) => {
        impl<'c> From<$type<'c>> for melior::ir::operation::Operation<'c> {
            fn from(op: $type<'c>) -> melior::ir::operation::Operation<'c> {
                unsafe { melior::ir::operation::Operation::from_raw(op.into_raw()) }
            }
        }
    };
    (@try_from $type:ident, $isa:ident, $opname:literal) => {
        impl<'c> TryFrom<melior::ir::operation::Operation<'c>> for $type<'c> {
            type Error = crate::error::Error;

            fn try_from(op: melior::ir::operation::Operation<'c>) -> Result<Self, Self::Error> {
                if unsafe { $isa(melior::ir::operation::OperationLike::to_raw(&op)) } {
                    Ok(unsafe { Self::from_raw(op.into_raw()) })
                } else {
                    Err(Self::Error::OperationExpected($opname, op.to_string()))
                }
            }
        }
    };
}

pub(crate) use concrete_op_type;

macro_rules! concrete_op_ref_type {
    ($type:ident, $isa:ident, $opname:literal) => {
        crate::macros::concrete_op_ref_type!(@def $type, concat!("Represents a non-owned reference to a '", $opname, "' op."));
        crate::macros::concrete_op_ref_type!(@impl $type,
            concat!("The MLIR operation must be a valid pointer of type ", stringify!($type) ,"."));
        crate::macros::concrete_op_ref_type!(@op_like $type);
        crate::macros::concrete_op_ref_type!(@display $type);
        crate::macros::concrete_op_ref_type!(@into $type);
        crate::macros::concrete_op_ref_type!(@try_from $type, $isa, $opname);
    };
    (@def $type:ident,  $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug)]
        pub struct $type<'c, 'a> {
            raw: mlir_sys::MlirOperation,
            _context: std::marker::PhantomData<&'a melior::ir::operation::Operation<'c>>,
        }
    };
    (@impl $type:ident,  $doc:expr) => {
        impl<'c, 'a> $type<'c, 'a> {
            /// # Safety
            #[doc = $doc]
            pub unsafe fn from_raw(raw: mlir_sys::MlirOperation) -> Self {
                Self {
                    raw,
                    _context: std::marker::PhantomData,
                }
            }
        }
    };
    (@op_like $type:ident) => {
        impl<'a, 'c: 'a> melior::ir::operation::OperationLike<'c, 'a> for $type<'c, 'a> {
            fn to_raw(&self) -> mlir_sys::MlirOperation {
                self.raw
            }
        }
    };
    (@display $type:ident) => {
        impl std::fmt::Display for $type<'_,'_> {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                let r = unsafe { melior::ir::operation::OperationRef::from_raw(self.raw) };
                std::fmt::Display::fmt(&r, formatter)
            }
        }
    };
    (@into $type:ident) => {
        impl<'c,'a> From<$type<'c, 'a>> for melior::ir::operation::OperationRef<'c,'a> {
            fn from(op: $type<'c,'a>) -> Self {
                unsafe { Self::from_raw(op.to_raw()) }
            }
        }
    };
    (@try_from $type:ident, $isa:ident, $opname:literal) => {
        impl<'c,'a> TryFrom<melior::ir::operation::OperationRef<'c,'a>> for $type<'c,'a> {
            type Error = crate::error::Error;

            fn try_from(op: melior::ir::operation::OperationRef<'c,'a>) -> Result<Self, Self::Error> {
                if unsafe { $isa(melior::ir::operation::OperationLike::to_raw(&op)) } {
                    Ok(unsafe { Self::from_raw(op.to_raw()) })
                } else {
                    Err(Self::Error::OperationExpected($opname, op.to_string()))
                }
            }
        }
    };
}

pub(crate) use concrete_op_ref_type;
