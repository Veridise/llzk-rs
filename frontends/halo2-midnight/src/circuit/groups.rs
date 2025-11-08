//! Bridge types for types in the [`midnight_proofs::circuit::groups`] module.

use crate::{
    macros::newtype,
    plonk::{_Any, _Column},
};
use halo2_frontend_core::{info_traits::GroupInfo, table::Cell, table::RegionIndex};
use midnight_proofs::circuit::groups::RegionsGroup;

//===----------------------------------------------------------------------===//
// RegionsGroup
//===----------------------------------------------------------------------===//

newtype!(RegionsGroup, _RegionsGroup with Debug);

impl GroupInfo for _RegionsGroup {
    fn inputs(&self) -> impl Iterator<Item = Cell> + '_ {
        self.0.inputs().map(|c| Cell {
            region_index: RegionIndex::from(*c.region_index),
            row_offset: c.row_offset,
            column: _Column::<_Any>::from(c.column).into(),
        })
    }

    fn outputs(&self) -> impl Iterator<Item = Cell> + '_ {
        self.0.outputs().map(|c| Cell {
            region_index: RegionIndex::from(*c.region_index),
            row_offset: c.row_offset,
            column: _Column::<_Any>::from(c.column).into(),
        })
    }
}
