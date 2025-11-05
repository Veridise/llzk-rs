use llzk_sys::{
    llzkCreateArrayOpBuildWithMapOperands, llzkCreateArrayOpBuildWithMapOperandsAndDims,
    llzkCreateArrayOpBuildWithValues,
};
use melior::ir::TypeLike;
use melior::ir::operation::OperationBuilder;
use melior::ir::{
    Attribute, AttributeLike, Location, Operation, Type, Value, ValueLike,
    attribute::DenseI32ArrayAttribute,
};
use mlir_sys::MlirOperation;

use crate::{
    builder::{OpBuilder, OpBuilderLike},
    value_range::ValueRange,
};

use super::ArrayType;

/// Possible constructors for creating `array.new` operations.
#[derive(Debug)]
pub enum ArrayCtor<'c, 'a, 'b, 'd> {
    /// Creates the array from a list of values.
    ///
    /// The list's length must be the same length as the array type.
    Values(&'a [Value<'c, 'b>]),
    /// Creates an empty array by specifying the values needed to instantiate
    /// AffineMap attributes used as dimension sizes in the result ArrayType.
    MapDimAttr(&'a [ValueRange<'c, 'a, 'b>], DenseI32ArrayAttribute<'c>),
    /// Creates an empty array by specifying the values needed to instantiate
    /// AffineMap attributes used as dimension sizes in the result ArrayType.
    MapDimSlice(&'a [ValueRange<'c, 'a, 'b>], &'d [i32]),
}

impl<'c, 'a, 'b, 'd> ArrayCtor<'c, 'a, 'b, 'd> {
    fn build(
        &self,
        builder: &OpBuilder<'c>,
        location: Location<'c>,
        r#type: ArrayType<'c>,
    ) -> MlirOperation {
        match self {
            Self::Values(values) => unsafe {
                let raw_values = values.iter().map(|v| v.to_raw()).collect::<Vec<_>>();
                llzkCreateArrayOpBuildWithValues(
                    builder.to_raw(),
                    location.to_raw(),
                    r#type.to_raw(),
                    raw_values.len() as isize,
                    raw_values.as_ptr(),
                )
            },

            Self::MapDimAttr(map_operands, num_dims_per_map) => unsafe {
                let raw_operands = map_operands.iter().map(|v| v.to_raw()).collect::<Vec<_>>();
                let dims: Attribute = (*num_dims_per_map).into();
                llzkCreateArrayOpBuildWithMapOperands(
                    builder.to_raw(),
                    location.to_raw(),
                    r#type.to_raw(),
                    raw_operands.len() as isize,
                    raw_operands.as_ptr(),
                    dims.to_raw(),
                )
            },

            Self::MapDimSlice(map_operands, num_dims_per_map) => unsafe {
                let raw_operands = map_operands.iter().map(|v| v.to_raw()).collect::<Vec<_>>();
                llzkCreateArrayOpBuildWithMapOperandsAndDims(
                    builder.to_raw(),
                    location.to_raw(),
                    r#type.to_raw(),
                    raw_operands.len() as isize,
                    raw_operands.as_ptr(),
                    num_dims_per_map.len() as isize,
                    num_dims_per_map.as_ptr(),
                )
            },
        }
    }
}

/// Creates an 'array.new' operation.
pub fn new<'c>(
    builder: &OpBuilder<'c>,
    location: Location<'c>,
    r#type: ArrayType<'c>,
    ctor: ArrayCtor<'c, '_, '_, '_>,
) -> Operation<'c> {
    unsafe { Operation::from_raw(ctor.build(builder, location, r#type)) }
}

fn read_like_op<'c>(
    name: &str,
    location: Location<'c>,
    result: Type<'c>,
    arr_ref: Value<'c, '_>,
    indices: &[Value<'c, '_>],
) -> Operation<'c> {
    OperationBuilder::new(name, location)
        .add_results(&[result])
        .add_operands(&[arr_ref])
        .add_operands(indices)
        .build()
        .expect("valid operation")
}

/// Creates an 'array.read' operation.
pub fn read<'c>(
    location: Location<'c>,
    result: Type<'c>,
    arr_ref: Value<'c, '_>,
    indices: &[Value<'c, '_>],
) -> Operation<'c> {
    read_like_op("array.read", location, result, arr_ref, indices)
}

/// Creates an 'array.extract' operation.
pub fn extract<'c>(
    location: Location<'c>,
    result: Type<'c>,
    arr_ref: Value<'c, '_>,
    indices: &[Value<'c, '_>],
) -> Operation<'c> {
    read_like_op("array.extract", location, result, arr_ref, indices)
}

fn write_like_op<'c>(
    name: &str,
    location: Location<'c>,
    arr_ref: Value<'c, '_>,
    indices: &[Value<'c, '_>],
    rvalue: Value<'c, '_>,
) -> Operation<'c> {
    OperationBuilder::new(name, location)
        .add_operands(&[arr_ref])
        .add_operands(indices)
        .add_operands(&[rvalue])
        .build()
        .expect("valid operation")
}

/// Creates an 'array.write' operation.
pub fn write<'c>(
    location: Location<'c>,
    arr_ref: Value<'c, '_>,
    indices: &[Value<'c, '_>],
    rvalue: Value<'c, '_>,
) -> Operation<'c> {
    write_like_op("array.write", location, arr_ref, indices, rvalue)
}

/// Creates an 'array.insert' operation.
pub fn insert<'c>(
    location: Location<'c>,
    arr_ref: Value<'c, '_>,
    indices: &[Value<'c, '_>],
    rvalue: Value<'c, '_>,
) -> Operation<'c> {
    write_like_op("array.insert", location, arr_ref, indices, rvalue)
}
