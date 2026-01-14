use crate::{
    attributes::NamedAttribute,
    builder::{OpBuilder, OpBuilderLike as _},
    dialect::r#struct::StructType,
    error::Error,
    macros::llzk_op_type,
    symbol_ref::{SymbolRefAttrLike, SymbolRefAttribute},
};

use llzk_sys::{
    llzkCallOpBuild, llzkCallOpGetCalleeIsCompute, llzkCallOpGetCalleeIsConstrain,
    llzkCallOpGetCalleeIsStructCompute, llzkCallOpGetCalleeIsStructConstrain,
    llzkCallOpGetSelfValueFromCompute, llzkCallOpGetSelfValueFromConstrain,
    llzkFuncDefOpCreateWithAttrsAndArgAttrs, llzkFuncDefOpGetFullyQualifiedName,
    llzkFuncDefOpGetHasAllowConstraintAttr, llzkFuncDefOpGetHasAllowWitnessAttr,
    llzkFuncDefOpGetHasArgIsPub, llzkFuncDefOpGetIsInStruct, llzkFuncDefOpGetIsStructCompute,
    llzkFuncDefOpGetIsStructConstrain, llzkFuncDefOpGetNameIsCompute,
    llzkFuncDefOpGetNameIsConstrain, llzkFuncDefOpGetSelfValueFromCompute,
    llzkFuncDefOpGetSelfValueFromConstrain, llzkFuncDefOpGetSingleResultTypeOfCompute,
    llzkFuncDefOpSetAllowConstraintAttr, llzkFuncDefOpSetAllowWitnessAttr, llzkOperationIsACallOp,
    llzkOperationIsAFuncDefOp,
};
use melior::{
    Context, StringRef,
    ir::{
        Attribute, AttributeLike, BlockLike as _, Location, Operation, RegionLike as _, Type,
        TypeLike, Value,
        attribute::{ArrayAttribute, TypeAttribute},
        block::BlockArgument,
        operation::{OperationBuilder, OperationLike, OperationMutLike},
        r#type::FunctionType,
    },
};
use mlir_sys::{MlirAttribute, MlirNamedAttribute, mlirDictionaryAttrGet, mlirNamedAttributeGet};

use std::ptr::null;

//===----------------------------------------------------------------------===//
// Helpers
//===----------------------------------------------------------------------===//

fn create_out_of_bounds_error<'c: 'a, 'a>(
    func: &(impl FuncDefOpLike<'c, 'a> + ?Sized),
    idx: usize,
) -> Error {
    match SymbolRefAttribute::try_from(func.fully_qualified_name()) {
        Ok(fqn) => Error::OutOfBoundsArgument(Some(fqn.to_string()), idx),
        Err(err) => err.into(),
    }
}

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

    /// Returns true if the function's name is [`FUNC_NAME_COMPUTE`](llzk_sys::FUNC_NAME_COMPUTE).
    fn name_is_compute(&self) -> bool {
        unsafe { llzkFuncDefOpGetNameIsCompute(self.to_raw()) }
    }

    /// Returns true if the function's name is [`FUNC_NAME_CONSTRAIN`](llzk_sys::FUNC_NAME_CONSTRAIN).
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

    /// If the function name is [`FUNC_NAME_COMPUTE`](llzk_sys::FUNC_NAME_COMPUTE), return the "self"
    /// value (i.e. the return value) from the function. Otherwise, return Err(ExpectedFunctionName).
    fn self_value_of_compute(&self) -> Result<Value<'c, 'a>, Error> {
        if self.name_is_compute() {
            Ok(unsafe { Value::from_raw(llzkFuncDefOpGetSelfValueFromCompute(self.to_raw())) })
        } else {
            Err(Error::ExpectedFunctionName(&llzk_sys::FUNC_NAME_COMPUTE))
        }
    }

    /// If the function name is [`FUNC_NAME_CONSTRAIN`](llzk_sys::FUNC_NAME_CONSTRAIN), return the "self"
    /// value (i.e. the first parameter) from the function. Otherwise, return Err(ExpectedFunctionName).
    fn self_value_of_constrain(&self) -> Result<Value<'c, 'a>, Error> {
        if self.name_is_constrain() {
            Ok(unsafe { Value::from_raw(llzkFuncDefOpGetSelfValueFromConstrain(self.to_raw())) })
        } else {
            Err(Error::ExpectedFunctionName(&llzk_sys::FUNC_NAME_CONSTRAIN))
        }
    }

    /// Assuming the function is the compute function returns its StructType result.
    fn result_type_of_compute(&self) -> StructType<'c> {
        unsafe { Type::from_raw(llzkFuncDefOpGetSingleResultTypeOfCompute(self.to_raw())) }
            .try_into()
            .expect("struct type")
    }

    /// Returns the n-th argument of the function.
    fn argument(&self, idx: usize) -> Result<BlockArgument<'c, 'a>, Error> {
        self.region(0)
            .map_err(Into::into)
            .and_then(|region| {
                region
                    .first_block()
                    .ok_or(create_out_of_bounds_error(self, idx))
            })
            .and_then(|block| block.argument(idx).map_err(Into::into))
    }

    /// Looks for an attribute in the n-th argument of the function.
    fn argument_attr(&self, idx: usize, name: &str) -> Result<Attribute<'c>, Error> {
        let arg_attrs: ArrayAttribute = self.attribute("arg_attrs")?.try_into()?;
        let arg = arg_attrs.element(idx)?;
        let name_ref = StringRef::new(name);
        unsafe {
            Attribute::from_option_raw(mlir_sys::mlirDictionaryAttrGetElementByName(
                arg.to_raw(),
                name_ref.to_raw(),
            ))
        }
        .ok_or_else(|| Error::AttributeNotFound(name.to_string()))
    }

    /// Get the [FunctionType] attribute.
    fn get_function_type_attribute(&self) -> Result<FunctionType<'c>, Error> {
        let attr = self.attribute("function_type")?;
        let type_attr: TypeAttribute<'c> = attr.try_into()?;
        let func_type: FunctionType<'c> = type_attr.value().try_into()?;
        Ok(func_type)
    }
}

/// Mutable operations for the `function.def` op.
pub trait FuncDefOpMutLike<'c: 'a, 'a>: FuncDefOpLike<'c, 'a> + OperationMutLike<'c, 'a> {}

//===----------------------------------------------------------------------===//
// FuncDefOp, FuncDefOpRef, and FuncDefOpRefMut
//===----------------------------------------------------------------------===//

llzk_op_type!(FuncDefOp, llzkOperationIsAFuncDefOp, "function.def");

impl<'a, 'c: 'a> FuncDefOpLike<'c, 'a> for FuncDefOp<'c> {}

impl<'a, 'c: 'a> FuncDefOpLike<'c, 'a> for FuncDefOpRef<'c, 'a> {}

impl<'a, 'c: 'a> FuncDefOpLike<'c, 'a> for FuncDefOpRefMut<'c, 'a> {}

impl<'a, 'c: 'a> FuncDefOpMutLike<'c, 'a> for FuncDefOp<'c> {}

impl<'a, 'c: 'a> FuncDefOpMutLike<'c, 'a> for FuncDefOpRefMut<'c, 'a> {}

//===----------------------------------------------------------------------===//
// CallOpLike
//===----------------------------------------------------------------------===//

/// Defines the public API of the 'function.call' op.
pub trait CallOpLike<'c: 'a, 'a>: OperationLike<'c, 'a> {
    /// Returns true if the call target name is [`FUNC_NAME_COMPUTE`](llzk_sys::FUNC_NAME_COMPUTE).
    fn callee_is_compute(&self) -> bool {
        unsafe { llzkCallOpGetCalleeIsCompute(self.to_raw()) }
    }

    /// Returns true if the call target name is [`FUNC_NAME_CONSTRAIN`](llzk_sys::FUNC_NAME_CONSTRAIN).
    fn callee_is_constrain(&self) -> bool {
        unsafe { llzkCallOpGetCalleeIsConstrain(self.to_raw()) }
    }

    /// Return `true` iff the callee function name is [`FUNC_NAME_COMPUTE`](llzk_sys::FUNC_NAME_COMPUTE) within a StructDefOp.
    fn callee_is_struct_compute(&self) -> bool {
        unsafe { llzkCallOpGetCalleeIsStructCompute(self.to_raw()) }
    }

    /// Return `true` iff the callee function name is [`FUNC_NAME_CONSTRAIN`](llzk_sys::FUNC_NAME_CONSTRAIN) within a StructDefOp.
    fn callee_is_struct_constrain(&self) -> bool {
        unsafe { llzkCallOpGetCalleeIsStructConstrain(self.to_raw()) }
    }

    /// If the function name is [`FUNC_NAME_COMPUTE`](llzk_sys::FUNC_NAME_COMPUTE), return the "self"
    /// value (i.e. the return value) from the callee function. Otherwise, return Err(ExpectedFunctionName).
    fn self_value_of_compute(&self) -> Result<Value<'c, 'a>, Error> {
        if self.callee_is_compute() {
            Ok(unsafe { Value::from_raw(llzkCallOpGetSelfValueFromCompute(self.to_raw())) })
        } else {
            Err(Error::ExpectedFunctionName(&llzk_sys::FUNC_NAME_COMPUTE))
        }
    }

    /// If the function name is [`FUNC_NAME_CONSTRAIN`](llzk_sys::FUNC_NAME_CONSTRAIN), return the "self"
    /// value (i.e. the first parameter) from the callee function. Otherwise, return Err(ExpectedFunctionName).
    fn self_value_of_constrain(&self) -> Result<Value<'c, 'a>, Error> {
        if self.callee_is_constrain() {
            Ok(unsafe { Value::from_raw(llzkCallOpGetSelfValueFromConstrain(self.to_raw())) })
        } else {
            Err(Error::ExpectedFunctionName(&llzk_sys::FUNC_NAME_CONSTRAIN))
        }
    }
}

//===----------------------------------------------------------------------===//
// CallOp, CallOpRef, CallOpRefMut
//===----------------------------------------------------------------------===//

llzk_op_type!(CallOp, llzkOperationIsACallOp, "function.call");

impl<'a, 'c: 'a> CallOpLike<'c, 'a> for CallOp<'c> {}

impl<'a, 'c: 'a> CallOpLike<'c, 'a> for CallOpRef<'c, 'a> {}

impl<'a, 'c: 'a> CallOpLike<'c, 'a> for CallOpRefMut<'c, 'a> {}

//===----------------------------------------------------------------------===//
// Operation factories
//===----------------------------------------------------------------------===//

fn tuple_to_named_attr((name, attr): &NamedAttribute) -> MlirNamedAttribute {
    unsafe { mlirNamedAttributeGet(name.to_raw(), attr.to_raw()) }
}

fn prepare_arg_attrs<'c>(
    arg_attrs: Option<&[Vec<NamedAttribute<'c>>]>,
    input_count: usize,
    ctx: &'c Context,
) -> Vec<MlirAttribute> {
    log::debug!("prepare_arg_attrs(\n{arg_attrs:?},\n{input_count},\n{ctx:?})");
    let Some(arg_attrs) = arg_attrs else {
        return vec![unsafe { mlirDictionaryAttrGet(ctx.to_raw(), 0, null()) }; input_count];
    };

    assert_eq!(arg_attrs.len(), input_count);
    arg_attrs
        .iter()
        .map(|arg_attr| {
            let named_attrs = Vec::from_iter(arg_attr.iter().map(tuple_to_named_attr));
            unsafe {
                mlirDictionaryAttrGet(
                    ctx.to_raw(),
                    named_attrs.len() as isize,
                    named_attrs.as_ptr(),
                )
            }
        })
        .collect()
}

/// Creates a 'function.def' operation. If the arg_attrs parameter is None creates as many empty
/// argument attributes as input arguments there are to satisfy the requirement of one
/// DictionaryAttr per argument.
pub fn def<'c>(
    location: Location<'c>,
    name: &str,
    r#type: FunctionType<'c>,
    attrs: &[NamedAttribute<'c>],
    arg_attrs: Option<&[Vec<NamedAttribute<'c>>]>,
) -> Result<FuncDefOp<'c>, Error> {
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
}

/// Return `true` iff the given op is `function.def`.
#[inline]
pub fn is_func_def<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "function.def")
}

/// Creates a new `function.call` operation.
pub fn call<'c>(
    builder: &OpBuilder<'c>,
    location: Location<'c>,
    name: impl SymbolRefAttrLike<'c>,
    args: &[Value<'c, '_>],
    return_types: &[impl TypeLike<'c>],
) -> Result<CallOp<'c>, Error> {
    unsafe {
        Operation::from_raw(llzkCallOpBuild(
            builder.to_raw(),
            location.to_raw(),
            return_types.len() as isize,
            return_types.as_ptr() as *const _,
            name.to_raw(),
            args.len() as isize,
            args.as_ptr() as *const _,
        ))
    }
    .try_into()
}

/// Return `true` iff the given op is `function.call`.
#[inline]
pub fn is_func_call<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "function.call")
}

/// Creates a new `function.return` operation.
///
/// This operation is the terminator op for `function.def` and must be the last operation of the
/// last block in it. The values array must match the number of outputs, and their types, of the
/// parent function.
pub fn r#return<'c>(location: Location<'c>, values: &[Value<'c, '_>]) -> Operation<'c> {
    OperationBuilder::new("function.return", location)
        .add_operands(values)
        .build()
        .unwrap()
}

/// Return `true` iff the given op is `function.return`.
#[inline]
pub fn is_func_return<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "function.return")
}
