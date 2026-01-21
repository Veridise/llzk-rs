//! `array` dialect operations and helper functions.
use llzk_sys::{llzkCreateArrayOpBuildWithMapOperands, llzkCreateArrayOpBuildWithValues};
use melior::ir::TypeLike;
use melior::ir::operation::OperationBuilder;
use melior::ir::{
    Location, Operation, Type, Value, ValueLike, attribute::DenseI32ArrayAttribute,
    operation::OperationLike,
};
use mlir_sys::MlirOperation;

use crate::map_operands::MapOperandsBuilder;
use crate::{
    builder::{OpBuilder, OpBuilderLike},
    value_ext::ValueRange,
};

use super::ArrayType;

/// Possible constructors for creating `array.new` operations.
#[derive(Debug)]
pub enum ArrayCtor<'c, 'a, 'b, 'd> {
    /// Creates the array from a list of values.
    ///
    /// The list length must be either zero or equal to the length of the flattened/linearized
    /// result ArrayType.
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
                let mut map_operands_builder = MapOperandsBuilder::new();

                for operands in *map_operands {
                    map_operands_builder.append_operands(*operands);
                }

                map_operands_builder.set_dims_per_map_from_attr(*num_dims_per_map);

                llzkCreateArrayOpBuildWithMapOperands(
                    builder.to_raw(),
                    location.to_raw(),
                    r#type.to_raw(),
                    map_operands_builder.to_raw(),
                )
            },

            Self::MapDimSlice(map_operands, num_dims_per_map) => unsafe {
                assert_eq!(map_operands.len(), num_dims_per_map.len());
                let mut map_operands_builder = MapOperandsBuilder::new();
                for (operands, dim) in std::iter::zip(*map_operands, *num_dims_per_map) {
                    map_operands_builder.append_operands_with_dim_count(*operands, *dim);
                }
                llzkCreateArrayOpBuildWithMapOperands(
                    builder.to_raw(),
                    location.to_raw(),
                    r#type.to_raw(),
                    map_operands_builder.to_raw(),
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

/// Return `true` iff the given op is `array.new`.
#[inline]
pub fn is_array_new<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "array.new")
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

/// Return `true` iff the given op is `array.read`.
#[inline]
pub fn is_array_read<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "array.read")
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

/// Return `true` iff the given op is `array.extract`.
#[inline]
pub fn is_array_extract<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "array.extract")
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

/// Return `true` iff the given op is `array.write`.
#[inline]
pub fn is_array_write<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "array.write")
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

/// Return `true` iff the given op is `array.insert`.
#[inline]
pub fn is_array_insert<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "array.insert")
}

/// Creates an 'array.len' operation.
pub fn len<'c>(
    location: Location<'c>,
    arr_ref: Value<'c, '_>,
    dim: Value<'c, '_>,
) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("array.len", location)
        .add_operands(&[arr_ref, dim])
        .add_results(&[Type::index(unsafe { ctx.to_ref() })])
        .build()
        .expect("valid operation")
}

/// Return `true` iff the given op is `array.len`.
#[inline]
pub fn is_array_len<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "array.len")
}
