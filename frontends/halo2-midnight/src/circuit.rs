//! Bridge types for types in the [`midnight_proofs::circuit`] module.

use midnight_proofs::circuit::RegionIndex;
pub mod groups;

//===----------------------------------------------------------------------===//
// RegionIndex
//===----------------------------------------------------------------------===//

crate::macros::newtype!(RegionIndex, _RegionIndex with Debug, Copy, Clone);

impl From<_RegionIndex> for halo2_llzk_frontend::RegionIndex {
    fn from(value: _RegionIndex) -> Self {
        (*value.0).into()
    }
}
