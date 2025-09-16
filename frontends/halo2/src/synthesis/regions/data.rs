use crate::halo2::*;
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

#[derive(Debug, Copy, Clone)]
pub enum RegionKind {
    Region,
    Table,
}

#[derive(Debug)]
pub struct RegionDataImpl {
    /// The name of the region. Not required to be unique.
    name: String,
    kind: RegionKind,
    index: Option<RegionIndex>,
    /// The selectors that have been enabled in this region. All other selectors are by
    /// construction not enabled.
    enabled_selectors: HashMap<Selector, Vec<usize>>,
    /// The columns involved in this region.
    columns: HashSet<Column<Any>>,
    /// The rows that this region starts and ends on, if known.
    rows: Option<(usize, usize)>,
    namespaces: Vec<String>,
}

impl RegionDataImpl {
    pub fn new<S: Into<String>>(name: S, index: RegionIndex) -> Self {
        Self {
            name: name.into(),
            kind: RegionKind::Region,
            index: Some(index),
            enabled_selectors: Default::default(),
            columns: Default::default(),
            rows: Default::default(),
            namespaces: Default::default(),
        }
    }

    pub fn enabled_selectors(&self) -> &HashMap<Selector, Vec<usize>> {
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

    pub fn selectors_enabled_for_row(&self, row: usize) -> Vec<&Selector> {
        self.enabled_selectors
            .iter()
            .filter(|(_, rows)| rows.contains(&row))
            .map(|(sel, _)| sel)
            .collect()
    }

    pub fn update_extent(&mut self, column: Column<Any>, row: usize) {
        self.columns.insert(column);
        self.rows = Some(
            self.rows
                .map_or_else(|| (row, row), |(start, end)| (start.min(row), end.max(row))),
        );
    }

    pub fn enable_selector(&mut self, s: Selector, row: usize) {
        self.enabled_selectors.entry(s).or_default().push(row);
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

#[derive(Copy, Clone, Debug)]
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

    pub fn kind(&self) -> RegionKind {
        self.inner.kind
    }

    pub fn selectors_enabled_for_row(&self, row: usize) -> Vec<&'a Selector> {
        self.inner.selectors_enabled_for_row(row)
    }

    pub fn enabled_selectors(&self) -> &'a HashMap<Selector, Vec<usize>> {
        self.inner.enabled_selectors()
    }

    pub fn columns(&self) -> &'a HashSet<Column<Any>> {
        &self.inner.columns
    }

    #[inline]
    pub fn header(&self) -> String {
        match (&self.kind(), &self.index()) {
            (RegionKind::Region, None) => format!("region <unk> {:?}", self.name()),
            (RegionKind::Region, Some(index)) => {
                format!("region {} {:?}", **index, self.name())
            }
            (RegionKind::Table, None) => format!("table {:?}", self.name()),
            _ => unreachable!(),
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
