use std::iter;

use melior::ir::{
    operation::OperationLike as _, BlockLike, BlockRef, OperationRef, RegionLike, RegionRef, Value,
    ValueLike,
};

pub fn block_list<'c: 'v, 'v>(
    r: impl RegionLike<'c, 'v>,
) -> impl Iterator<Item = BlockRef<'c, 'v>> {
    iter::successors(r.first_block(), |b| b.next_in_region())
}

pub fn operations_list<'c: 'v, 'v>(
    b: impl BlockLike<'c, 'v>,
) -> impl Iterator<Item = OperationRef<'c, 'v>> {
    iter::successors(b.first_operation(), |op| op.next_in_block())
}

pub fn box_value<'c: 'v, 'v>(v: impl ValueLike<'c> + 'v) -> Box<dyn ValueLike<'c> + 'v> {
    Box::new(v)
}

pub fn unbox_value<'c, 'v>(b: &Box<dyn ValueLike<'c>>) -> Value<'c, 'v> {
    unsafe { Value::from_raw(b.to_raw()) }
}
