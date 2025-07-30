use crate::{
    backend::{
        func::{ArgNo, FieldId, FuncIO},
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    halo2::*,
    io::IOCell,
    synthesis::regions::FQN,
    CircuitIO,
};
use anyhow::{bail, Result};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
    marker::PhantomData,
    ops::{AddAssign, Range, RangeFrom},
};

use super::{BlanketFills, SharedRegionData, TableData};

#[derive(Debug, Default)]
struct RegionDataInner<F> {
    /// Constant values assigned to fixed columns in the region.
    fixed: HashMap<(usize, usize), Value<F>>,
    /// Represents the circuit filling rows with a single value.
    /// Row start offsets are maintained in chronological order, so when
    /// querying a row the latest that matches is the correct value.
    blanket_fills: HashMap<usize, BlanketFills<F>>,
    /// The selectors that have been enabled in this region. All other selectors are by
    /// construction not enabled.
    enabled_selectors: HashMap<Selector, Vec<usize>>,
    /// The columns involved in this region.
    columns: HashSet<Column<Any>>,
    /// The rows that this region starts and ends on, if known.
    rows: Option<(usize, usize)>,
    namespaces: Vec<String>,
}

#[derive(Debug, Copy, Clone)]
pub(super) enum RegionKind {
    Region,
    Table,
}

#[derive(Debug)]
pub struct RegionDataImpl<F> {
    /// The name of the region. Not required to be unique.
    name: String,
    kind: RegionKind,
    index: Option<RegionIndex>,
    inner: RegionDataInner<F>,
    shared: Option<SharedRegionData>,
}

impl<F: Default + Clone + Copy + std::fmt::Debug> RegionDataImpl<F> {
    pub fn new<S: Into<String>>(name: S, index: RegionIndex) -> Self {
        Self {
            name: name.into(),
            kind: RegionKind::Region,
            index: Some(index),
            inner: Default::default(),
            shared: Some(Default::default()),
        }
    }

    pub fn kind(&self) -> RegionKind {
        self.kind
    }

    pub fn enabled_selectors(&self) -> &HashMap<Selector, Vec<usize>> {
        &self.inner.enabled_selectors
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn index(&self) -> Option<RegionIndex> {
        self.index
    }

    pub fn take_shared(&mut self) -> Option<SharedRegionData> {
        self.shared.take()
    }

    pub fn mark_as_table(&mut self) {
        self.index = None;
        self.kind = RegionKind::Table;
    }

    pub fn selectors_enabled_for_row(&self, row: usize) -> Vec<&Selector> {
        self.inner
            .enabled_selectors
            .iter()
            .filter(|(_, rows)| rows.contains(&row))
            .map(|(sel, _)| sel)
            .collect()
    }

    pub fn blanket_fill(&mut self, column: Column<Fixed>, row: usize, value: Value<F>) {
        self.inner
            .blanket_fills
            .entry(column.index())
            .or_default()
            .push((row.., value));
        self.update_extent(column.into(), row);
    }

    pub fn update_extent(&mut self, column: Column<Any>, row: usize) {
        self.inner.columns.insert(column);

        // The region start is the earliest row assigned to.
        // The region end is the latest row assigned to.
        let (mut start, mut end) = self.inner.rows.unwrap_or((row, row));
        if row < start {
            // The first row assigned was not at start 0 within the region.
            start = row;
        }
        if row > end {
            end = row;
        }
        self.inner.rows = Some((start, end));
    }

    pub fn enable_selector(&mut self, s: Selector, row: usize) {
        self.inner.enabled_selectors.entry(s).or_default().push(row);
    }

    pub fn assign_fixed<VR>(&mut self, fixed: Column<Fixed>, row: usize, value: Value<VR>)
    where
        F: Field,
        VR: Into<Assigned<F>>,
    {
        let value = value.map(|vr| vr.into());
        log::debug!(
            "Recording fixed assignment @ col = {}, row = {row}, value = {value:?}",
            fixed.index()
        );
        self.inner
            .fixed
            .insert((fixed.index(), row), value.map(|vr| vr.evaluate()));
    }

    fn resolve_from_blanket_fills(&self, column: usize, row: usize) -> Option<Value<F>> {
        self.inner
            .blanket_fills
            .get(&column)
            .and_then(|values| values.iter().rfind(|(range, _)| range.contains(&row)))
            .map(|(_, v)| *v)
    }

    pub fn resolve_fixed(&self, column: usize, row: usize) -> Option<Value<F>> {
        log::debug!("Fixed values: {:?}", self.inner.fixed);
        self.inner
            .fixed
            .get(&(column, row))
            .inspect(|v| {
                log::debug!(
                    "[Region {}] For ({column}, {row}) we got value {v:?}",
                    self.name
                )
            })
            .cloned()
            .or_else(|| self.resolve_from_blanket_fills(column, row))
    }

    pub fn rows(&self) -> Range<usize> {
        self.inner
            .rows
            .map(|(begin, end)| begin..end + 1)
            .unwrap_or(0..0)
    }

    pub fn note_advice(&mut self, column: Column<Advice>, row: usize, name: String) {
        let fqn = FQN::new(
            self.name.as_str(),
            self.index,
            &self.inner.namespaces,
            name.into(),
        );
        log::debug!(
            "Recording advice assignment @ col = {}, row = {row}, name = {fqn}",
            column.index()
        );
        self.shared
            .as_mut()
            .unwrap()
            .advice_names_mut()
            .insert((column.index(), row), fqn);
    }

    pub fn push_namespace<NR, N>(&mut self, name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.inner.namespaces.push(name().into())
    }

    pub fn pop_namespace(&mut self, name: Option<String>) {
        match name {
            Some(name) => {
                if let Some(idx) = self.inner.namespaces.iter().rposition(|e| *e == name) {
                    self.inner.namespaces.remove(idx);
                }
            }
            None => {
                self.inner.namespaces.pop();
            }
        }
    }
}

impl<F: Copy + Default> From<RegionDataImpl<F>> for TableData<F> {
    fn from(value: RegionDataImpl<F>) -> Self {
        Self::new(value.inner.fixed, value.inner.blanket_fills)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RegionData<'a, F> {
    pub shared: &'a SharedRegionData,
    pub inner: &'a RegionDataImpl<F>,
}

impl<'a, F: Default + Clone + Copy + std::fmt::Debug> RegionData<'a, F> {
    pub fn new(shared: &'a SharedRegionData, inner: &'a RegionDataImpl<F>) -> Self {
        Self { shared, inner }
    }

    pub fn find_advice_name(&self, col: usize, row: usize) -> Cow<'a, FQN> {
        self.shared
            .advice_names()
            .get(&(col, row))
            .map(Cow::Borrowed)
            .unwrap_or_else(|| {
                Cow::Owned(FQN::new(
                    self.inner.name.as_str(),
                    self.inner.index,
                    &[],
                    None,
                ))
            })
    }

    pub fn rows(&self) -> Range<usize> {
        self.inner.rows()
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn find_fixed_col_assignment(&self, col: Column<Fixed>, row: usize) -> Option<Value<F>> {
        self.inner.resolve_fixed(col.index(), row)
    }
}
