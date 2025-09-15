use crate::halo2::*;
use data::RegionDataImpl;
use std::{
    collections::{HashMap, HashSet},
    ops::RangeFrom,
};

pub(super) mod data;
mod fixed;
mod region_row;
mod row;
mod table;

pub use data::RegionData;

pub use fixed::FixedData;
pub use region_row::{RegionRow, RegionRowLike};
pub use row::Row;
pub use table::TableData;

type BlanketFills<F> = Vec<(RangeFrom<usize>, Value<F>)>;

pub type RegionIndexToStart = HashMap<RegionIndex, RegionStart>;

/// A set of regions
#[derive(Default, Debug)]
pub struct Regions {
    regions: Vec<RegionDataImpl>,
    current: Option<RegionDataImpl>,
    // If we need to transform the previous region into a table we store the index here to
    // reuse it.
    recovered_index: Option<RegionIndex>,
    last_is_table: bool,
}

impl Regions {
    /// Adds a new region.
    pub fn push<NR, N>(
        &mut self,
        region_name: N,
        next_index: &mut dyn Iterator<Item = RegionIndex>,
        tables: &mut Vec<HashSet<Column<Fixed>>>,
    ) where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        assert!(self.current.is_none());
        self.move_latest_to_tables(tables);
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

    pub fn regions<'a>(&'a self) -> Vec<RegionData<'a>> {
        self.regions.iter().map(RegionData::new).collect()
    }

    /// Marks the last region as a table.
    ///
    /// Panics if there is a currently active region or there is already a recovered index.
    pub fn mark_region(&mut self) {
        assert!(
            self.current.is_none(),
            "Cannot move the last region to tables list while we have another active region"
        );
        self.last_is_table = true;
    }

    /// Moves the last commited region to the tables vector.
    fn move_latest_to_tables(&mut self, tables: &mut Vec<HashSet<Column<Fixed>>>) {
        if !self.last_is_table {
            return;
        }
        self.last_is_table = false;
        log::debug!("Regions: {:?}", self.regions);
        let table = self.regions.pop();
        if table.is_none() {
            log::debug!("Region list was empty");
            return;
        }
        let mut table = table.unwrap();
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
}
