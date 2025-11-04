//! Opaque module that exposes the correct halo2 library based on the implementation selected via
//! feature flags.

//#[cfg(not(feature = "midnight"))]
//pub use halo2curves::bn256;

//#[cfg(feature = "axiom")]
//mod axiom;
//#[cfg(feature = "midnight")]
mod midnight;
//#[cfg(feature = "pse")]
//mod pse;
//#[cfg(feature = "pse-v1")]
//mod pse_v1;
//#[cfg(feature = "scroll")]
//mod scroll;
//#[cfg(feature = "zcash")]
//mod zcash;

//#[cfg(feature = "axiom")]
//pub use axiom::*;
//#[cfg(feature = "midnight")]
pub use midnight::*;

use crate::{
    info_traits::{GroupInfo, QueryInfo, SelectorInfo},
    synthesis::regions::RegionIndex,
    table::{Cell, Rotation, RotationExt},
};
//#[cfg(feature = "pse")]
//pub use pse::*;
//#[cfg(feature = "pse-v1")]
//pub use pse_v1::*;
//#[cfg(feature = "scroll")]
//pub use scroll::*;
//#[cfg(feature = "zcash")]
//pub use zcash::*;

impl RotationExt<Halo2Rotation> for Rotation {
    fn cur() -> Halo2Rotation {
        Halo2Rotation::cur()
    }

    #[cfg(test)]
    fn next() -> Halo2Rotation {
        Halo2Rotation::next()
    }
}

/// Temporary implementation of [`QueryInfo`] for [`halo2_proofs::plonk::FixedQuery`]
impl QueryInfo for halo2_proofs::plonk::FixedQuery {
    type Kind = crate::resolvers::Fixed;

    fn rotation(&self) -> Rotation {
        self.rotation().0
    }

    fn column_index(&self) -> usize {
        self.column_index()
    }
}

/// Temporary implementation of [`QueryInfo`] for [`halo2_proofs::plonk::AdviceQuery`]
impl QueryInfo for halo2_proofs::plonk::AdviceQuery {
    type Kind = crate::resolvers::Advice;

    fn rotation(&self) -> Rotation {
        self.rotation().0
    }

    fn column_index(&self) -> usize {
        self.column_index()
    }
}

/// Temporary implementation of [`QueryInfo`] for [`halo2_proofs::plonk::InstanceQuery`]
impl QueryInfo for halo2_proofs::plonk::InstanceQuery {
    type Kind = crate::resolvers::Instance;

    fn rotation(&self) -> Rotation {
        self.rotation().0
    }

    fn column_index(&self) -> usize {
        self.column_index()
    }
}

/// Temporary implementation of [`SelectorInfo`] for [`halo2_proofs::plonk::Selector`]
impl SelectorInfo for halo2_proofs::plonk::Selector {
    fn id(&self) -> usize {
        self.index()
    }
}

/// Temporary implementation of [`GroupInfo`] for [`halo2_proofs::circuit::groups::RegionsGroup`].
impl GroupInfo for halo2_proofs::circuit::groups::RegionsGroup {
    fn inputs(&self) -> impl Iterator<Item = Cell> + '_ {
        self.inputs().map(|c| Cell {
            region_index: RegionIndex::from(*c.region_index),
            row_offset: c.row_offset,
            column: c.column.into(),
        })
    }

    fn outputs(&self) -> impl Iterator<Item = Cell> + '_ {
        self.outputs().map(|c| Cell {
            region_index: RegionIndex::from(*c.region_index),
            row_offset: c.row_offset,
            column: c.column.into(),
        })
    }
}
