use std::iter;

use melior::ir::{BlockLike, BlockRef, OperationRef, RegionLike, operation::OperationLike as _};

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
