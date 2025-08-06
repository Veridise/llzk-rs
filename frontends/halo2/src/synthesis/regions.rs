use crate::halo2::*;
use data::RegionDataImpl;
use std::ops::RangeFrom;

mod data;
mod fixed;
mod fqn;
mod region_row;
mod row;
mod shared;
mod table;

pub use data::RegionData;
pub use fqn::FQN;
pub use region_row::{RegionRow, RegionRowLike};
pub use row::Row;
pub use shared::SharedRegionData;
pub use table::TableData;

type BlanketFills<F> = Vec<(RangeFrom<usize>, Value<F>)>;

#[derive(Default, Debug)]
pub struct Regions<F: Copy + std::fmt::Debug> {
    shared: SharedRegionData<F>,
    regions: Vec<RegionDataImpl<F>>,
    tables: Vec<TableData<F>>,
    current: Option<RegionDataImpl<F>>,
    current_is_table: bool,
}

impl<F: Default + Clone + Copy + std::fmt::Debug> Regions<F> {
    pub fn push<NR, N>(&mut self, region_name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        // The last region turned out to be a table. Remove it from the regions list before adding
        // the new one.
        if self.current_is_table {
            if let Some(mut table) = self.regions.pop() {
                log::debug!(
                    "Demoting region {} {:?} to table",
                    table
                        .index()
                        .as_deref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "<unk>".to_owned()),
                    table.name()
                );
                table.mark_as_table();
                self.tables.push(table.into());
            }
        }

        assert!(self.current.is_none());
        let name: String = region_name().into();
        let index = self.regions.len();
        log::debug!("Region {index} {name:?} is the current region");
        self.current = Some(RegionDataImpl::new(name, index.into()));
        self.current_is_table = false;
    }

    pub fn commit(&mut self) {
        let mut region = self.current.take().unwrap();

        if self.current_is_table {
            log::debug!(
                "Region {} {:?} is a table",
                *region.index().unwrap(),
                region.name()
            );
            region.mark_as_table();
            self.tables.push(region.into());
        } else {
            log::debug!(
                "Region {} {:?} added to the regions list",
                *region.index().unwrap(),
                region.name()
            );
            self.shared += region.take_shared();
            self.regions.push(region);
        }
    }

    pub fn edit<FN, FR>(&mut self, f: FN) -> Option<FR>
    where
        FN: FnOnce(&mut RegionDataImpl<F>) -> FR,
    {
        if let Some(region) = self.current.as_mut() {
            return Some(f(region));
        }
        None
    }

    pub fn edit_current_or_last<FN, FR>(&mut self, f: FN) -> Option<FR>
    where
        FN: FnOnce(&mut RegionDataImpl<F>) -> FR,
    {
        if let Some(region) = self.current.as_mut() {
            return Some(f(region));
        }
        if let Some(region) = self.regions.first_mut() {
            return Some(f(region));
        }
        None
    }

    pub fn regions<'a>(&'a self) -> Vec<RegionData<'a, F>> {
        self.regions
            .iter()
            .map(|inner| RegionData::new(&self.shared, inner))
            .collect()
    }

    pub fn tables(&self) -> &[TableData<F>] {
        &self.tables
    }

    pub fn mark_current_as_table(&mut self) {
        self.current_is_table = true;
    }

    pub fn seen_advice_cells(&self) -> impl Iterator<Item = (&(usize, usize), &FQN)> {
        self.shared.advice_names().iter()
    }
}
