/// Defines an operation type for a concrete operation along with reference types.
///
/// Mimics [`Operation`] and the reference types mimic [`OperationRef`] and [`OperationRefMut`].
///
/// Not all operations need to be defined. Only the ones that have operations beyond the basics.
macro_rules! llzk_op_type {
    ($type:ident, $isa:ident, $opname:literal) => {
        // Owned type

        #[doc = concat!("Represents an owned '", $opname, "' op.")]
        pub struct $type<'c> {
            raw: mlir_sys::MlirOperation,
            _context: std::marker::PhantomData<&'c melior::Context>,
        }

        impl<'c> $type<'c> {
            /// # Safety
            #[doc = concat!("The MLIR operation must be a valid pointer of type ", stringify!($type) ,".")]
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

            #[doc = concat!("Creates an optional operation from a raw object of type '", $opname, "'.")]
            ///
            /// # Safety
            ///
            /// A raw object must be valid.
            pub fn from_option_raw(raw: mlir_sys::MlirOperation) -> Option<Self> {
                if raw.ptr.is_null() && unsafe { $isa(raw) } {
                    None
                } else {
                   unsafe { Some(Self::from_raw(raw)) }
                }
            }
        }

        impl<'a, 'c: 'a> melior::ir::operation::OperationLike<'c, 'a> for $type<'c> {
            fn to_raw(&self) -> mlir_sys::MlirOperation {
                self.raw
            }
        }

        impl<'c: 'a, 'a> melior::ir::operation::OperationMutLike<'c, 'a> for $type<'c> {}

        impl Clone for $type<'_> {
            fn clone(&self) -> Self {
                unsafe { Self::from_raw(mlir_sys::mlirOperationClone(self.raw)) }
            }
        }

        impl Drop for $type<'_> {
            fn drop(&mut self) {
                unsafe { mlir_sys::mlirOperationDestroy(self.raw) };
            }
        }

        impl<'a, 'c: 'a, T: melior::ir::operation::OperationLike<'c, 'a>> PartialEq<T> for $type<'c> {
            fn eq(&self, other: &T) -> bool {
                unsafe { mlir_sys::mlirOperationEqual(self.raw, other.to_raw()) }
            }
        }

        impl Eq for $type<'_> {}

        impl std::fmt::Display for $type<'_> {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                let r = unsafe { melior::ir::operation::OperationRef::from_raw(self.raw) };
                std::fmt::Display::fmt(&r, formatter)
            }
        }

        impl std::fmt::Debug for $type<'_> {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                writeln!(formatter, "{}(", stringify!($type))?;
                std::fmt::Display::fmt(self, formatter)?;
                write!(formatter, ")")
            }
        }

        impl<'c> From<$type<'c>> for melior::ir::operation::Operation<'c> {
            fn from(op: $type<'c>) -> melior::ir::operation::Operation<'c> {
                unsafe { melior::ir::operation::Operation::from_raw(op.into_raw()) }
            }
        }

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

        // Reference type
        paste::paste! {
            #[doc = concat!("Represents a non-owned reference to a '", $opname, "' op.")]
            #[derive(Copy, Clone)]
            pub struct  [<$type Ref>]<'c, 'a> {
                raw: mlir_sys::MlirOperation,
                _context: std::marker::PhantomData<&'a melior::ir::operation::Operation<'c>>,
            }


            impl<'c, 'a> [<$type Ref>]<'c, 'a> {
                #[doc = concat!("Returns an operation of type '", $opname, "' op.")]
                ///
                /// This function is different from `deref` because the correct lifetime is
                /// kept for the return type.
                ///
                /// # Safety
                ///
                /// The returned reference is safe to use only in the lifetime scope of the
                /// operation reference.
                pub unsafe fn to_ref(&self) -> &'a $type<'c> {
                    // As we can't deref OperationRef<'a> into `&'a Operation`, we forcibly cast its
                    // lifetime here to extend it from the lifetime of `ObjectRef<'a>` itself into
                    // `'a`.
                    unsafe { std::mem::transmute(self) }
                }

                /// Converts an operation reference into a raw object.
                pub const fn to_raw(self) -> mlir_sys::MlirOperation {
                    self.raw
                }

                /// Creates an operation reference from a raw object.
                ///
                /// # Safety
                ///
                #[doc = concat!("The MLIR operation must be a valid pointer of type ", stringify!([<$type Ref>]) ,".")]
                pub unsafe fn from_raw(raw: mlir_sys::MlirOperation) -> Self {
                    Self {
                        raw,
                        _context: std::marker::PhantomData,
                    }
                }

                /// Creates an optional operation reference from a raw object.
                ///
                /// # Safety
                ///
                #[doc = concat!("The MLIR operation must be a valid pointer of type ", stringify!([<$type Ref>]) ,".")]
                pub  fn from_option_raw(raw: mlir_sys::MlirOperation) -> Option<Self> {
                    if raw.ptr.is_null() || unsafe { !$isa(raw) } {
                        None
                    } else {
                        unsafe { Some(Self::from_raw(raw)) }
                    }
                }
            }

            impl<'a, 'c: 'a> melior::ir::operation::OperationLike<'c, 'a> for [<$type Ref>]<'c, 'a> {
                fn to_raw(&self) -> mlir_sys::MlirOperation {
                    self.raw
                }
            }

            impl<'c> std::ops::Deref for [<$type Ref>]<'c, '_> {
                type Target = $type<'c>;

                fn deref(&self) -> &Self::Target {
                    unsafe { self.to_ref() }
                }
            }

            impl<'a, 'c: 'a, T: melior::ir::operation::OperationLike<'c, 'a>> PartialEq<T> for [<$type Ref>]<'c, 'a> {
                fn eq(&self, other: &T) -> bool {
                    unsafe { mlir_sys::mlirOperationEqual(self.raw, other.to_raw()) }
                }
            }

            impl Eq for [<$type Ref>]<'_, '_> {}

            impl std::fmt::Display for [<$type Ref>]<'_,'_> {
                fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    use std::ops::Deref;
                    std::fmt::Display::fmt(self.deref(), formatter)
                }
            }

            impl std::fmt::Debug for [<$type Ref>]<'_,'_> {
                fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    use std::ops::Deref;
                    std::fmt::Debug::fmt(self.deref(), formatter)
                }
            }

            impl<'c,'a> From<[<$type Ref>]<'c, 'a>> for melior::ir::operation::OperationRef<'c,'a> {
                fn from(op: [<$type Ref>]<'c,'a>) -> Self {
                    unsafe { Self::from_raw(op.to_raw()) }
                }
            }

            impl<'c,'a> TryFrom<melior::ir::operation::OperationRef<'c,'a>> for [<$type Ref>]<'c,'a> {
                type Error = crate::error::Error;

                fn try_from(op: melior::ir::operation::OperationRef<'c,'a>) -> Result<Self, Self::Error> {
                    if unsafe { $isa(melior::ir::operation::OperationLike::to_raw(&op)) } {
                        Ok(unsafe { Self::from_raw(op.to_raw()) })
                    } else {
                        Err(Self::Error::OperationExpected($opname, op.to_string()))
                    }
                }
            }

            // Mutable reference type

            /// A mutable reference to an operation.
            #[derive(Clone, Copy)]
            pub struct [<$type RefMut>]<'c, 'a> {
                raw: mlir_sys::MlirOperation,
                _reference: std::marker::PhantomData<&'a melior::ir::operation::Operation<'c>>,
            }

            impl [<$type RefMut>]<'_, '_> {
                /// Converts an operation reference into a raw object.
                pub const fn to_raw(self) -> mlir_sys::MlirOperation {
                    self.raw
                }

                /// Creates an operation reference from a raw object.
                ///
                /// # Safety
                ///
                #[doc = concat!("A raw object must be valid and of type '", $opname, "'.")]
                pub unsafe fn from_raw(raw: mlir_sys::MlirOperation) -> Self {
                    Self {
                        raw,
                        _reference: Default::default(),
                    }
                }

                /// Creates an optional operation reference from a raw object.
                ///
                /// # Safety
                ///
                /// A raw object must be valid.
                pub  fn from_option_raw(raw: mlir_sys::MlirOperation) -> Option<Self> {
                    if raw.ptr.is_null() || unsafe { $isa(raw) } {
                        None
                    } else {
                       unsafe { Some(Self::from_raw(raw)) }
                    }
                }
            }

            impl<'c, 'a> melior::ir::operation::OperationLike<'c, 'a> for [<$type RefMut>]<'c, 'a> {
                fn to_raw(&self) -> mlir_sys::MlirOperation {
                    self.raw
                }
            }

            impl<'c, 'a> melior::ir::operation::OperationMutLike<'c, 'a> for [<$type RefMut>]<'c, 'a> {}

            impl<'c> std::ops::Deref for [<$type RefMut>]<'c, '_> {
                type Target = $type<'c>;

                fn deref(&self) -> &Self::Target {
                    unsafe { std::mem::transmute(self) }
                }
            }

            impl std::ops::DerefMut for [<$type RefMut>]<'_, '_> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    unsafe { std::mem::transmute(self) }
                }
            }

            impl<'a, 'c: 'a, T: melior::ir::operation::OperationLike<'c, 'a>> PartialEq<T> for [<$type RefMut>]<'c, 'a> {
                fn eq(&self, other: &T) -> bool {
                    unsafe { mlir_sys::mlirOperationEqual(self.raw, other.to_raw()) }
                }
            }

            impl Eq for [<$type RefMut>]<'_, '_> {}

            impl std::fmt::Display for [<$type RefMut>]<'_, '_> {
                fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    use std::ops::Deref;
                    std::fmt::Display::fmt(self.deref(), formatter)
                }
            }

            impl std::fmt::Debug for [<$type RefMut>]<'_, '_> {
                fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    use std::ops::Deref;
                    std::fmt::Debug::fmt(self.deref(), formatter)
                }
            }

            impl<'c,'a> From<[<$type RefMut>]<'c, 'a>> for melior::ir::operation::OperationRefMut<'c,'a> {
                fn from(op: [<$type RefMut>]<'c,'a>) -> Self {
                    unsafe { Self::from_raw(op.to_raw()) }
                }
            }

            impl<'c,'a> TryFrom<melior::ir::operation::OperationRefMut<'c,'a>> for [<$type RefMut>]<'c,'a> {
                type Error = crate::error::Error;

                fn try_from(op: melior::ir::operation::OperationRefMut<'c,'a>) -> Result<Self, Self::Error> {
                    if unsafe { $isa(melior::ir::operation::OperationLike::to_raw(&op)) } {
                        Ok(unsafe { Self::from_raw(op.to_raw()) })
                    } else {
                        Err(Self::Error::OperationExpected($opname, op.to_string()))
                    }
                }
            }
        }
    };
}

pub(crate) use llzk_op_type;
