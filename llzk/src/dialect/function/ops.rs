use crate::{
    builder::{OpBuilder, OpBuilderLike as _},
    dialect::r#struct::StructType,
    error::Error,
    macros::llzk_op_type,
    symbol_ref::SymbolRefAttribute,
};
use llzk_sys::{
    llzkCallOpBuild, llzkFuncDefOpCreateWithAttrsAndArgAttrs, llzkFuncDefOpGetFullyQualifiedName,
    llzkFuncDefOpGetHasAllowConstraintAttr, llzkFuncDefOpGetHasAllowWitnessAttr,
    llzkFuncDefOpGetHasArgIsPub, llzkFuncDefOpGetIsInStruct, llzkFuncDefOpGetIsStructCompute,
    llzkFuncDefOpGetIsStructConstrain, llzkFuncDefOpGetNameIsCompute,
    llzkFuncDefOpGetNameIsConstrain, llzkFuncDefOpGetSingleResultTypeOfCompute,
    llzkFuncDefOpSetAllowConstraintAttr, llzkFuncDefOpSetAllowWitnessAttr, llzkOperationIsACallOp,
    llzkOperationIsAFuncDefOp,
};
use melior::{
    Context, StringRef,
    ir::{
        Attribute, AttributeLike, BlockLike as _, Identifier, Location, Operation, RegionLike as _,
        Type, TypeLike, Value,
        attribute::{ArrayAttribute, FlatSymbolRefAttribute},
        block::BlockArgument,
        operation::{OperationBuilder, OperationLike},
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
}

//===----------------------------------------------------------------------===//
// FuncDefOp, FuncDefOpRef, and FuncDefOpRefMut
//===----------------------------------------------------------------------===//

llzk_op_type!(FuncDefOp, llzkOperationIsAFuncDefOp, "function.def");

impl<'a, 'c: 'a> FuncDefOpLike<'c, 'a> for FuncDefOp<'c> {}

impl<'a, 'c: 'a> FuncDefOpLike<'c, 'a> for FuncDefOpRef<'c, 'a> {}

impl<'a, 'c: 'a> FuncDefOpLike<'c, 'a> for FuncDefOpRefMut<'c, 'a> {}

//===----------------------------------------------------------------------===//
// CallOpLike
//===----------------------------------------------------------------------===//

/// Defines the public API of the 'function.call' op.
pub trait CallOpLike<'c: 'a, 'a>: OperationLike<'c, 'a> {}

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

fn tuple_to_named_attr(t: &(Identifier, Attribute)) -> MlirNamedAttribute {
    unsafe { mlirNamedAttributeGet(t.0.to_raw(), t.1.to_raw()) }
}

fn prepare_arg_attrs<'c>(
    arg_attrs: Option<&[&[(Identifier<'c>, Attribute<'c>)]]>,
    input_count: usize,
    ctx: &'c Context,
) -> Vec<MlirAttribute> {
    log::debug!("prepare_arg_attrs(\n{arg_attrs:?},\n{input_count},\n{ctx:?})");
    if let Some(arg_attrs) = arg_attrs {
        assert_eq!(arg_attrs.len(), input_count);
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
            .inspect(|a| log::debug!("attribute = {a:?}"))
            .collect()
    }
}

/// Creates a 'function.def' operation. If the arg_attrs parameter is None creates as many empty
/// argument attributes as input arguments there are to satisfy the requirement of one
/// DictionaryAttr per argument.
pub fn def<'c>(
    location: Location<'c>,
    name: &str,
    r#type: FunctionType<'c>,
    attrs: &[(Identifier<'c>, Attribute<'c>)],
    arg_attrs: Option<&[&[(Identifier<'c>, Attribute<'c>)]]>,
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

/// Creates a new `function.call` operation.
pub fn call<'c>(
    builder: &OpBuilder<'c>,
    location: Location<'c>,
    name: &str,
    args: &[Value<'c, '_>],
    return_type: impl TypeLike<'c>,
) -> Result<CallOp<'c>, Error> {
    let ctx = location.context();
    let name = FlatSymbolRefAttribute::new(unsafe { ctx.to_ref() }, name);
    unsafe {
        Operation::from_raw(llzkCallOpBuild(
            builder.to_raw(),
            location.to_raw(),
            1 as isize,
            // &return_type.to_raw(),
            [return_type].as_ptr() as *const _,
            name.to_raw(),
            args.len() as isize,
            args.as_ptr() as *const _,
        ))
    }
    .try_into()
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
