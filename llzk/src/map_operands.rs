//! Types for working with affine map operands.

use llzk_sys::{
    LlzkAffineMapOperandsBuilder, llzkAffineMapOperandsBuilderAppendDimCount,
    llzkAffineMapOperandsBuilderAppendOperands,
    llzkAffineMapOperandsBuilderAppendOperandsWithDimCount,
    llzkAffineMapOperandsBuilderConvertDimsPerMapToArray,
    llzkAffineMapOperandsBuilderConvertDimsPerMapToAttr, llzkAffineMapOperandsBuilderCreate,
    llzkAffineMapOperandsBuilderDestroy, llzkAffineMapOperandsBuilderGetDimsPerMapAttr,
    llzkAffineMapOperandsBuilderSetDimsPerMapFromAttr,
};
use melior::{
    Context,
    ir::{Attribute, AttributeLike as _, attribute::DenseI32ArrayAttribute},
};

use crate::value_ext::ValueRange;

#[derive(Debug)]
/// Wrapper type for [`LlzkAffineMapOperandsBuilder`]
pub struct MapOperandsBuilder {
    raw: LlzkAffineMapOperandsBuilder,
}

impl MapOperandsBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self {
            raw: unsafe { llzkAffineMapOperandsBuilderCreate() },
        }
    }

    /// Returns the low level representation of the builder.
    pub unsafe fn to_raw(&self) -> LlzkAffineMapOperandsBuilder {
        self.raw
    }

    /// Appends a set of operands for an affine map.
    ///
    /// The operands in this range are considered the operands of a single map.
    pub fn append_operands<'slf, 'val>(&'slf mut self, operands: ValueRange<'_, 'val, '_>)
    where
        'val: 'slf,
    {
        unsafe { llzkAffineMapOperandsBuilderAppendOperands(&mut self.raw, 1, &operands.to_raw()) }
    }

    /// Appends a dimension count.
    pub fn append_dim_count(&mut self, dim: i32) {
        unsafe {
            llzkAffineMapOperandsBuilderAppendDimCount(&mut self.raw, 1, &dim);
        }
    }

    /// Appends a set of operands for an affine map along with the number of them that are
    /// dimensions.
    ///
    /// The operands in this range are considered the operands of a single map.
    pub fn append_operands_with_dim_count<'slf, 'val>(
        &'slf mut self,
        operands: ValueRange<'_, 'val, '_>,
        dim: i32,
    ) where
        'val: 'slf,
    {
        unsafe {
            llzkAffineMapOperandsBuilderAppendOperandsWithDimCount(
                &mut self.raw,
                1,
                &operands.to_raw(),
                &dim,
            );
        }
    }

    /// Sets the number of dimensions from an array attribute.
    pub fn set_dims_per_map_from_attr<'ctx, 'slf>(
        &'slf mut self,
        attr: DenseI32ArrayAttribute<'ctx>,
    ) where
        'ctx: 'slf,
    {
        unsafe { llzkAffineMapOperandsBuilderSetDimsPerMapFromAttr(&mut self.raw, attr.to_raw()) }
    }

    /// Converts the inner representation of the dimensions to an array.
    pub fn convert_dims_per_map_to_array(&mut self) {
        unsafe { llzkAffineMapOperandsBuilderConvertDimsPerMapToArray(&mut self.raw) }
    }

    /// Converts the inner representation of the dimensions to a [`DenseI32ArrayAttribute`].
    pub fn convert_dims_per_map_to_attr<'ctx, 'slf>(&'slf mut self, context: &'ctx Context)
    where
        'ctx: 'slf,
    {
        unsafe {
            llzkAffineMapOperandsBuilderConvertDimsPerMapToAttr(&mut self.raw, context.to_raw())
        }
    }

    /// Returns the dimensions as an attribute.
    pub fn get_dims_per_map_attr<'ctx, 'slf>(
        &'slf self,
        context: &'ctx Context,
    ) -> DenseI32ArrayAttribute<'ctx> {
        DenseI32ArrayAttribute::try_from(unsafe {
            Attribute::from_option_raw(llzkAffineMapOperandsBuilderGetDimsPerMapAttr(
                self.raw,
                context.to_raw(),
            ))
            .unwrap()
        })
        .unwrap()
    }
}

impl Drop for MapOperandsBuilder {
    fn drop(&mut self) {
        unsafe {
            llzkAffineMapOperandsBuilderDestroy(&mut self.raw);
        }
    }
}
