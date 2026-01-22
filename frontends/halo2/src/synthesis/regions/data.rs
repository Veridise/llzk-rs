use crate::gates::SelectorSet;
use halo2_frontend_core::{
    info_traits::SelectorInfo,
    table::{Any, Column, ColumnType, RegionIndex, RegionStart},
};

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Range,
};

pub struct RegionDataImpl {
    /// The name of the region. Not required to be unique.
    name: String,
    index: Option<RegionIndex>,
    /// The selectors that have been enabled in this region. All other selectors are by
    /// construction not enabled.
    enabled_selectors: HashMap<usize, SelectorSet>,
    /// The columns involved in this region.
    columns: HashSet<Column<Any>>,
    /// The rows that this region starts and ends on, if known.
    rows: Option<(usize, usize)>,
    namespaces: Vec<String>,
}

impl RegionDataImpl {
    pub fn new<S: Into<String>>(
        name: S,
        index: RegionIndex,
        region_start: Option<RegionStart>,
    ) -> Self {
        Self {
            name: name.into(),
            index: Some(index),
            enabled_selectors: Default::default(),
            columns: Default::default(),
            rows: region_start.map(|start| (*start, *start)),
            namespaces: Default::default(),
        }
    }

    pub fn enabled_selectors(&self) -> &HashMap<usize, SelectorSet> {
        &self.enabled_selectors
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn index(&self) -> Option<RegionIndex> {
        self.index
    }

    /// The first row of the region.
    pub fn start(&self) -> Option<usize> {
        self.rows.map(|(start, _)| start)
    }

    pub fn index_as_str(&self) -> String {
        self.index()
            .as_deref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "<unk>".to_owned())
    }

    /// Takes the index from the region and leaves it without one.
    pub fn take_index(&mut self) -> Option<RegionIndex> {
        self.index.take()
    }

    pub fn selectors_enabled_for_row(&self, row: usize) -> Cow<'_, SelectorSet> {
        self.enabled_selectors
            .get(&row)
            .map(Cow::Borrowed)
            .unwrap_or_default()
    }

    pub fn update_extent(&mut self, column: Column<Any>, row: usize) {
        log::info!(
            "[Region '{}'] Updating extent with column = {column:?} and row = {row}",
            self.name()
        );
        self.columns.insert(column);
        self.rows = Some(
            self.rows
                .map_or_else(|| (row, row), |(start, end)| (start.min(row), end.max(row))),
        );

        log::info!(
            "[Region '{}'] Updated extent rows = {:?} | columns = {:?}",
            self.name(),
            self.rows(),
            self.columns,
        );
    }

    pub fn enable_selector(&mut self, s: &dyn SelectorInfo, row: usize) {
        self.enabled_selectors
            .entry(row)
            .or_default()
            .insert(s.id());
    }

    pub fn rows(&self) -> Range<usize> {
        self.rows.map(|(begin, end)| begin..end + 1).unwrap_or(0..0)
    }

    pub fn push_namespace<NR, N>(&mut self, name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.namespaces.push(name().into())
    }

    pub fn pop_namespace(&mut self, name: Option<String>) {
        match name {
            Some(name) => {
                if let Some(idx) = self.namespaces.iter().rposition(|e| *e == name) {
                    self.namespaces.remove(idx);
                }
            }
            None => {
                self.namespaces.pop();
            }
        }
    }

    pub fn columns<C>(&self) -> HashSet<Column<C>>
    where
        Column<C>: TryFrom<Column<Any>>,
        C: ColumnType + std::hash::Hash,
    {
        self.columns
            .iter()
            .filter_map(|c| (*c).try_into().ok())
            .collect()
    }
}

impl std::fmt::Debug for RegionDataImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Region \"{}\" ({})", self.name, self.index_as_str())?;
        writeln!(f, "  Rows {:?}", self.rows())?;
        write!(f, "  Columns ")?;
        crate::utils::fmt_columns(&self.columns, f)?;
        writeln!(f)?;
        writeln!(f, "  Selectors")?;
        for (row, selectors) in &self.enabled_selectors {
            let mut bitvec = 0_usize;
            for elt in selectors {
                bitvec |= 1 << elt;
            }
            writeln!(f, "    {row:10}: {bitvec:0b}")?;
        }
        writeln!(f)
    }
}

#[derive(Copy, Clone)]
pub struct RegionData<'a> {
    inner: &'a RegionDataImpl,
}

impl<'a> RegionData<'a> {
    pub fn new(inner: &'a RegionDataImpl) -> Self {
        Self { inner }
    }

    pub fn rows(&self) -> Range<usize> {
        self.inner.rows()
    }

    pub fn name(&self) -> &'a str {
        &self.inner.name
    }

    pub fn index(&self) -> Option<RegionIndex> {
        self.inner.index
    }

    pub fn start(&self) -> Option<usize> {
        self.inner.start()
    }

    pub fn selectors_enabled_for_row(&self, row: usize) -> Cow<'_, SelectorSet> {
        self.inner.selectors_enabled_for_row(row)
    }

    pub fn enabled_selectors(&self) -> &'a HashMap<usize, SelectorSet> {
        self.inner.enabled_selectors()
    }

    pub fn columns(&self) -> &'a HashSet<Column<Any>> {
        &self.inner.columns
    }

    #[inline]
    pub fn header(&self) -> String {
        match &self.index() {
            None => format!("region <unk> {:?}", self.name()),
            Some(index) => {
                format!("region {} {:?}", **index, self.name())
            }
        }
    }

    /// Returns the offset from the start of the region, only if the row is within bounds.  
    pub fn relativize(&self, row: usize) -> Option<usize> {
        let rows = self.inner.rows();
        if rows.contains(&row) {
            Some(row - rows.start)
        } else {
            None
        }
    }
}

impl std::fmt::Debug for RegionData<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.inner, f)
    }
}
