use data::RegionDataImpl;
use halo2_frontend_core::{
    query::Fixed,
    table::{Column, RegionIndex},
};

use std::{collections::HashSet, ops::Deref};

pub(super) mod data;
mod fixed;
mod region_row;
mod row;
mod table;

pub use data::RegionData;

pub use fixed::FixedData;
pub use region_row::RegionRow;
pub use row::Row;
pub use table::TableData;

pub use halo2_frontend_core::table::RegionStart;

/// A set of regions
#[derive(Default, Debug)]
pub struct Regions {
    regions: Vec<RegionDataImpl>,
    current: Option<RegionDataImpl>,
    // If we need to transform the previous region into a table we store the index here to
    // reuse it.
    recovered_index: Option<RegionIndex>,
    last_is_table: bool,
    // Set of already used indices.
    // The automatic index assignment will skip these and the tool will panic
    // if a manually assigned index has already been used.
    used_indices: HashSet<RegionIndex>,
}

impl Regions {
    /// Adds a new region.
    pub fn push<NR, N>(
        &mut self,
        region_name: N,
        next_index: &mut dyn Iterator<Item = RegionIndex>,
        tables: &mut Vec<HashSet<Column<Fixed>>>,
        region_index: Option<RegionIndex>,
        region_start: Option<RegionStart>,
    ) where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        assert!(self.current.is_none());
        self.move_latest_to_tables(tables);
        let name: String = region_name().into();
        let index = self.get_next_index(next_index, region_index);
        log::debug!("Region {} {name:?} is the current region", *index);
        self.current = Some(RegionDataImpl::new(name, index, region_start));
    }

    fn get_next_index(
        &mut self,
        next_index: &mut dyn Iterator<Item = RegionIndex>,
        region_index: Option<RegionIndex>,
    ) -> RegionIndex {
        // The index is either not passed or is fresh.
        assert!(region_index.is_none_or(|index| !self.used_indices.contains(&index)));
        let new_index = region_index.unwrap_or_else(|| {
            self
            // Reuse the previous index if available.
            .recovered_index
            .take()
            // Otherwise request a new one.
            .unwrap_or_else(|| {
                next_index.skip_while(|index| {
                        self.used_indices.contains(index)
                    })
                    .next()
                    .expect("Iterator of region indices should be infinite")
            })
        });
        self.used_indices.insert(new_index);
        new_index
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
        let indices = self
            .regions
            .iter()
            .map(|r| r.index().unwrap())
            .collect::<HashSet<_>>();
        assert_eq!(self.regions.len(), indices.len());
        assert_eq!(self.regions.len(), self.used_indices.len());
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
