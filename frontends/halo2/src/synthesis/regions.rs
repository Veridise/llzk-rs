use crate::halo2::*;
use data::RegionDataImpl;
use std::{
    collections::{HashMap, HashSet},
    ops::RangeFrom,
};

pub(super) mod data;
mod fixed;
mod fqn;
mod region_row;
mod row;
mod shared;
mod table;

pub use data::RegionData;
pub use fixed::FixedData;
pub use fqn::FQN;
pub use region_row::{RegionRow, RegionRowLike};
pub use row::Row;
//pub use shared::SharedRegionData;
pub use table::TableData;

type BlanketFills<F> = Vec<(RangeFrom<usize>, Value<F>)>;

pub type RegionIndexToStart = HashMap<RegionIndex, RegionStart>;

/// A set of regions
#[derive(Default, Debug)]
pub struct Regions {
    //shared: SharedRegionData<F>,
    regions: Vec<RegionDataImpl>,
    current: Option<RegionDataImpl>,
    // If we need to transform the previous region into a table we store the index here to
    // reuse it.
    recovered_index: Option<RegionIndex>,
}

impl Regions {
    /// Adds a new region.
    pub fn push<NR, N>(&mut self, region_name: N, next_index: &mut dyn Iterator<Item = RegionIndex>)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        assert!(self.current.is_none());
        let name: String = region_name().into();
        let index = self
            // Reuse the previous index if available.
            .recovered_index
            .take()
            // Otherwise request a new one.
            .unwrap_or_else(|| {
                next_index
                    .next()
                    .expect("Iterator of region indices should be infinite")
            });
        log::debug!("Region {} {name:?} is the current region", *index);
        self.current = Some(RegionDataImpl::new(name, index));
    }

    /// Commits the current region to the list of regions.
    pub fn commit(&mut self) {
        let region = self.current.take().unwrap();
        log::debug!(
            "Region {} {:?} added to the regions list",
            *region.index().unwrap(),
            region.name()
        );
        self.regions.push(region);
    }

    pub fn edit<FN, FR>(&mut self, f: FN) -> Option<FR>
    where
        FN: FnOnce(&mut RegionDataImpl) -> FR,
    {
        if let Some(region) = self.current.as_mut() {
            return Some(f(region));
        }
        None
    }

    //pub fn edit_current_or_last<FN, FR>(&mut self, f: FN) -> Option<FR>
    //where
    //    FN: FnOnce(&mut RegionDataImpl<F>) -> FR,
    //{
    //    if let Some(region) = self.current.as_mut() {
    //        return Some(f(region));
    //    }
    //    if let Some(region) = self.regions.first_mut() {
    //        return Some(f(region));
    //    }
    //    None
    //}

    pub fn regions<'a>(&'a self) -> Vec<RegionData<'a>> {
        self.regions
            .iter()
            .map(|inner| RegionData::new(inner))
            .collect()
    }

    /// Moves the last commited region to the tables vector.
    ///
    /// Panics if there is a currently active region or there is already a recovered index.
    pub fn move_latest_to_tables(&mut self, tables: &mut Vec<HashSet<Column<Fixed>>>) {
        assert!(
            self.current.is_none(),
            "Cannot move the last region to tables list while we have another active region"
        );
        let mut table = self
            .regions
            .pop()
            .expect("Cannot move to the tables list because list is empty");
        log::debug!(
            "Demoting region {} {:?} to table",
            table.index_as_str(),
            table.name()
        );
        assert!(
            self.recovered_index.is_none(),
            "There is already a recovered index"
        );
        self.recovered_index = table.take_index();
        tables.push(table.columns());
    }

    //pub fn seen_advice_cells(&self) -> impl Iterator<Item = (&(usize, usize), &FQN)> {
    //    self.shared.advice_names().iter()
    //}
}
