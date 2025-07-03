use std::{fmt, ops::Deref, ptr::null};

use llzk_sys::{
    llzkFuncDefOpCreateWithAttrsAndArgAttrs, llzkFuncDefOpGetFullyQualifiedName,
    llzkFuncDefOpGetHasAllowConstraintAttr, llzkFuncDefOpGetHasAllowWitnessAttr,
    llzkFuncDefOpGetHasArgIsPub, llzkFuncDefOpGetIsInStruct, llzkFuncDefOpGetIsStructCompute,
    llzkFuncDefOpGetIsStructConstrain, llzkFuncDefOpGetNameIsCompute,
    llzkFuncDefOpGetNameIsConstrain, llzkFuncDefOpGetSingleResultTypeOfCompute,
    llzkFuncDefOpSetAllowConstraintAttr, llzkFuncDefOpSetAllowWitnessAttr, llzkOperationIsACallOp,
    llzkOperationIsAFuncDefOp,
};
use melior::{
    ir::{
        operation::{OperationBuilder, OperationLike},
        r#type::FunctionType,
        Attribute, AttributeLike, Identifier, Location, Operation, OperationRef, Type, TypeLike,
        Value,
    },
    Context, StringRef,
};
use mlir_sys::{
    mlirDictionaryAttrGet, mlirNamedAttributeGet, MlirAttribute, MlirNamedAttribute, MlirOperation,
};

use crate::{dialect::r#struct::StructType, error::Error};

//===----------------------------------------------------------------------===//
// FuncDefOpLike
//===----------------------------------------------------------------------===//

/// Defines the public API of the 'function.def' op.
pub trait FuncDefOpLike<'c: 'a, 'a>: OperationLike<'c, 'a> {
    /// Returns true if the FuncDefOp has the allow_constraint attribute.
    fn has_allow_constraint_attr(&self) -> bool {
        unsafe { llzkFuncDefOpGetHasAllowConstraintAttr(self.to_raw()) }
    }

    /// Sets the allow_constraint attribute in the FuncDefOp operation.
    fn set_allow_constraint_attr(&self, value: bool) {
        unsafe { llzkFuncDefOpSetAllowConstraintAttr(self.to_raw(), value) }
    }

    /// Returns true if the FuncDefOp has the allow_witness attribute.
    fn has_allow_witness_attr(&self) -> bool {
        unsafe { llzkFuncDefOpGetHasAllowWitnessAttr(self.to_raw()) }
    }

    /// Sets the allow_witness attribute in the FuncDefOp operation.
    fn set_allow_witness_attr(&self, value: bool) {
        unsafe { llzkFuncDefOpSetAllowWitnessAttr(self.to_raw(), value) }
    }

    /// Returns true if the `idx`-th argument has the Pub attribute.
    fn arg_is_pub(&self, idx: u32) -> bool {
        unsafe { llzkFuncDefOpGetHasArgIsPub(self.to_raw(), idx) }
    }

    /// Returns the fully qualified name of the function.
    fn fully_qualified_name(&self) -> Attribute<'c> {
        unsafe { Attribute::from_raw(llzkFuncDefOpGetFullyQualifiedName(self.to_raw())) }
    }

    /// Returns true if the function's name is 'compute'.
    fn name_is_compute(&self) -> bool {
        unsafe { llzkFuncDefOpGetNameIsCompute(self.to_raw()) }
    }

    /// Returns true if the function's name is 'constrain'.
    fn name_is_constrain(&self) -> bool {
        unsafe { llzkFuncDefOpGetNameIsConstrain(self.to_raw()) }
    }

    /// Returns true if the function's defined inside a struct.
    fn is_in_struct(&self) -> bool {
        unsafe { llzkFuncDefOpGetIsInStruct(self.to_raw()) }
    }

    /// Returns true if the function is the struct's witness computation.
    fn is_struct_compute(&self) -> bool {
        unsafe { llzkFuncDefOpGetIsStructCompute(self.to_raw()) }
    }

    /// Returns true if the function is the struct's constrain definition.
    fn is_struct_constrain(&self) -> bool {
        unsafe { llzkFuncDefOpGetIsStructConstrain(self.to_raw()) }
    }

    /// Assuming the function is the compute function returns its StructType result.
    fn result_type_of_compute(&self) -> StructType<'c> {
        unsafe { Type::from_raw(llzkFuncDefOpGetSingleResultTypeOfCompute(self.to_raw())) }
            .try_into()
            .expect("struct type")
    }
}

//===----------------------------------------------------------------------===//
// FuncDefOp
//===----------------------------------------------------------------------===//

/// Represents an owned 'function.def' op.
pub struct FuncDefOp<'c> {
    inner: Operation<'c>,
}

impl FuncDefOp<'_> {
    /// # Safety
    /// The MLIR operation must be a valid pointer of type llzk::function::FuncDefOp.
    pub unsafe fn from_raw(raw: MlirOperation) -> Self {
        unsafe {
            Self {
                inner: Operation::from_raw(raw),
            }
        }
    }
}

impl<'a, 'c: 'a> OperationLike<'c, 'a> for FuncDefOp<'c> {
    fn to_raw(&self) -> MlirOperation {
        self.inner.to_raw()
    }
}

impl<'a, 'c: 'a> FuncDefOpLike<'c, 'a> for FuncDefOp<'c> {}

impl fmt::Display for FuncDefOp<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'c> Deref for FuncDefOp<'c> {
    type Target = Operation<'c>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'c> From<FuncDefOp<'c>> for Operation<'c> {
    fn from(op: FuncDefOp<'c>) -> Operation<'c> {
        op.inner
    }
}

impl<'c> TryFrom<Operation<'c>> for FuncDefOp<'c> {
    type Error = Error;

    fn try_from(op: Operation<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkOperationIsAFuncDefOp(op.to_raw()) } {
            Ok(unsafe { Self::from_raw(op.to_raw()) })
        } else {
            Err(Self::Error::OperationExpected(
                "function.def",
                op.to_string(),
            ))
        }
    }
}

//===----------------------------------------------------------------------===//
// FuncDefOpRef
//===----------------------------------------------------------------------===//

/// Represents a non-owned reference to a 'function.def' op.
#[derive(Clone, Copy)]
pub struct FuncDefOpRef<'c, 'a> {
    inner: OperationRef<'c, 'a>,
}

impl FuncDefOpRef<'_, '_> {
    /// # Safety
    /// The MLIR operation must be a valid pointer of type llzk::function::FuncDefOp.
    pub unsafe fn from_raw(raw: MlirOperation) -> Self {
        unsafe {
            Self {
                inner: OperationRef::from_raw(raw),
            }
        }
    }
}

impl<'a, 'c: 'a> OperationLike<'c, 'a> for FuncDefOpRef<'c, 'a> {
    fn to_raw(&self) -> MlirOperation {
        self.inner.to_raw()
    }
}

impl<'a, 'c: 'a> FuncDefOpLike<'c, 'a> for FuncDefOpRef<'c, 'a> {}

impl fmt::Display for FuncDefOpRef<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'a, 'c: 'a> Deref for FuncDefOpRef<'c, 'a> {
    type Target = OperationRef<'c, 'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, 'c: 'a> From<FuncDefOpRef<'c, 'a>> for OperationRef<'c, 'a> {
    fn from(op: FuncDefOpRef<'c, 'a>) -> OperationRef<'c, 'a> {
        op.inner
    }
}

impl<'a, 'c: 'a> TryFrom<OperationRef<'c, 'a>> for FuncDefOpRef<'c, 'a> {
    type Error = Error;

    fn try_from(op: OperationRef<'c, 'a>) -> Result<Self, Self::Error> {
        if unsafe { llzkOperationIsAFuncDefOp(op.to_raw()) } {
            Ok(unsafe { Self::from_raw(op.to_raw()) })
        } else {
            Err(Self::Error::OperationExpected(
                "function.def",
                op.to_string(),
            ))
        }
    }
}

//===----------------------------------------------------------------------===//
// CallOpLike
//===----------------------------------------------------------------------===//

/// Defines the public API of the 'function.call' op.
pub trait CallOpLike<'c: 'a, 'a>: OperationLike<'c, 'a> {}

//===----------------------------------------------------------------------===//
// CallOp
//===----------------------------------------------------------------------===//

/// Represents an owned 'function.call' op.
pub struct CallOp<'c> {
    inner: Operation<'c>,
}

impl CallOp<'_> {
    /// # Safety
    /// The MLIR operation must be a valid pointer of type llzk::function::CallOp.
    pub unsafe fn from_raw(raw: MlirOperation) -> Self {
        unsafe {
            Self {
                inner: Operation::from_raw(raw),
            }
        }
    }
}

impl<'a, 'c: 'a> OperationLike<'c, 'a> for CallOp<'c> {
    fn to_raw(&self) -> MlirOperation {
        self.inner.to_raw()
    }
}

impl<'a, 'c: 'a> CallOpLike<'c, 'a> for CallOp<'c> {}

impl fmt::Display for CallOp<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'c> Deref for CallOp<'c> {
    type Target = Operation<'c>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'c> From<CallOp<'c>> for Operation<'c> {
    fn from(op: CallOp<'c>) -> Operation<'c> {
        op.inner
    }
}

impl<'c> TryFrom<Operation<'c>> for CallOp<'c> {
    type Error = Error;

    fn try_from(op: Operation<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkOperationIsACallOp(op.to_raw()) } {
            Ok(unsafe { Self::from_raw(op.to_raw()) })
        } else {
            Err(Self::Error::OperationExpected(
                "function.call",
                op.to_string(),
            ))
        }
    }
}

//===----------------------------------------------------------------------===//
// CallOpRef
//===----------------------------------------------------------------------===//

/// Represents a non-owned reference to a 'function.call' op.
#[derive(Clone, Copy)]
pub struct CallOpRef<'c, 'a> {
    inner: OperationRef<'c, 'a>,
}

impl CallOpRef<'_, '_> {
    /// # Safety
    /// The MLIR operation must be a valid pointer of type llzk::function::CallOp
    pub unsafe fn from_raw(raw: MlirOperation) -> Self {
        unsafe {
            Self {
                inner: OperationRef::from_raw(raw),
            }
        }
    }
}

impl<'a, 'c: 'a> OperationLike<'c, 'a> for CallOpRef<'c, 'a> {
    fn to_raw(&self) -> MlirOperation {
        self.inner.to_raw()
    }
}

impl<'a, 'c: 'a> CallOpLike<'c, 'a> for CallOpRef<'c, 'a> {}

impl fmt::Display for CallOpRef<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'a, 'c: 'a> Deref for CallOpRef<'c, 'a> {
    type Target = OperationRef<'c, 'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, 'c: 'a> From<CallOpRef<'c, 'a>> for OperationRef<'c, 'a> {
    fn from(op: CallOpRef<'c, 'a>) -> OperationRef<'c, 'a> {
        op.inner
    }
}

impl<'a, 'c: 'a> TryFrom<OperationRef<'c, 'a>> for CallOpRef<'c, 'a> {
    type Error = Error;

    fn try_from(op: OperationRef<'c, 'a>) -> Result<Self, Self::Error> {
        if unsafe { llzkOperationIsACallOp(op.to_raw()) } {
            Ok(unsafe { Self::from_raw(op.to_raw()) })
        } else {
            Err(Self::Error::OperationExpected(
                "function.call",
                op.to_string(),
            ))
        }
    }
}

//===----------------------------------------------------------------------===//
// operation factories
//===----------------------------------------------------------------------===//

fn tuple_to_named_attr(t: &(Identifier, Attribute)) -> MlirNamedAttribute {
    unsafe { mlirNamedAttributeGet(t.0.to_raw(), t.1.to_raw()) }
}

fn prepare_arg_attrs<'c>(
    arg_attrs: Option<&[&[(Identifier<'c>, Attribute<'c>)]]>,
    input_count: usize,
    ctx: &'c Context,
) -> Vec<MlirAttribute> {
    if let Some(arg_attrs) = arg_attrs {
        arg_attrs
            .iter()
            .map(|arg_attr| {
                let named_attrs = arg_attr.iter().map(tuple_to_named_attr).collect::<Vec<_>>();
                unsafe {
                    mlirDictionaryAttrGet(
                        ctx.to_raw(),
                        named_attrs.len() as isize,
                        named_attrs.as_ptr(),
                    )
                }
            })
            .collect()
    } else {
        (0..input_count)
            .map(|_| unsafe { mlirDictionaryAttrGet(ctx.to_raw(), 0, null()) })
            .collect()
    }
}

/// Creates a 'function.def' operation. If the arg_attrs parameter is None creates as many empty argument
/// attributes as input arguments there are to satisfy the requirement of one DictionaryAttr per
/// argument.
pub fn def<'c>(
    location: Location<'c>,
    name: &str,
    r#type: FunctionType<'c>,
    attrs: &[(Identifier<'c>, Attribute<'c>)],
    arg_attrs: Option<&[&[(Identifier<'c>, Attribute<'c>)]]>,
) -> FuncDefOp<'c> {
    let ctx = location.context();
    let name = StringRef::new(name);
    let attrs: Vec<_> = attrs.iter().map(tuple_to_named_attr).collect();
    let arg_attrs = prepare_arg_attrs(arg_attrs, r#type.input_count(), unsafe { ctx.to_ref() });
    unsafe {
        Operation::from_raw(llzkFuncDefOpCreateWithAttrsAndArgAttrs(
            location.to_raw(),
            name.to_raw(),
            r#type.to_raw(),
            attrs.len() as isize,
            attrs.as_ptr(),
            arg_attrs.len() as isize,
            arg_attrs.as_ptr(),
        ))
    }
    .try_into()
    .expect("op of type 'function.def'")
}

pub fn call<'c>() -> CallOp<'c> {
    todo!()
}

pub fn r#return<'c>(location: Location<'c>, values: &[Value<'c, '_>]) -> Operation<'c> {
    OperationBuilder::new("function.return", location)
        .add_operands(values)
        .build()
        .unwrap()
}
