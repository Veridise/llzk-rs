use crate::{halo2::*, synthesis::regions::FQN};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Range,
};

use super::TableData;

#[derive(Debug, Default)]
struct RegionDataInner {
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
    inner: RegionDataInner,
}

impl RegionDataImpl {
    pub fn new<S: Into<String>>(name: S, index: RegionIndex) -> Self {
        Self {
            name: name.into(),
            kind: RegionKind::Region,
            index: Some(index),
            inner: Default::default(),
        }
    }

    //pub fn kind(&self) -> RegionKind {
    //    self.kind
    //}

    pub fn enabled_selectors(&self) -> &HashMap<Selector, Vec<usize>> {
        &self.inner.enabled_selectors
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn index(&self) -> Option<RegionIndex> {
        self.index
    }

    /// The first row of the region.
    pub fn start(&self) -> Option<usize> {
        self.inner.rows.map(|(start, _)| start)
    }

    pub fn index_as_str(&self) -> String {
        self.index()
            .as_deref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "<unk>".to_owned())
    }

    pub fn into_table<F: Field>(self) -> TableData<F> {
        todo!()
    }

    //pub fn take_shared(&mut self) -> SharedRegionData<F> {
    //    self.shared.clone()
    //}

    /// Takes the index from the region and leaves it without one.
    pub fn take_index(&mut self) -> Option<RegionIndex> {
        self.index.take()
    }

    pub fn selectors_enabled_for_row(&self, row: usize) -> Vec<&Selector> {
        self.inner
            .enabled_selectors
            .iter()
            .filter(|(_, rows)| rows.contains(&row))
            .map(|(sel, _)| sel)
            .collect()
    }

    pub fn update_extent(&mut self, column: Column<Any>, row: usize) {
        self.inner.columns.insert(column);
        self.inner.rows = Some(
            self.inner
                .rows
                .map_or_else(|| (row, row), |(start, end)| (start.min(row), end.max(row))),
        );
    }

    pub fn enable_selector(&mut self, s: Selector, row: usize) {
        self.inner.enabled_selectors.entry(s).or_default().push(row);
    }

    pub fn rows(&self) -> Range<usize> {
        self.inner
            .rows
            .map(|(begin, end)| begin..end + 1)
            .unwrap_or(0..0)
    }

    //pub fn note_advice(&mut self, column: Column<Advice>, row: usize, name: String) {
    //    let fqn = FQN::new(
    //        self.name.as_str(),
    //        self.index,
    //        &self.inner.namespaces,
    //        name.into(),
    //    );
    //    log::debug!(
    //        "Recording advice assignment @ col = {}, row = {row}, name = {fqn}",
    //        column.index()
    //    );
    //    self.shared
    //        .advice_names_mut()
    //        .insert((column.index(), row), fqn);
    //}

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

    pub fn columns<C>(&self) -> HashSet<Column<C>>
    where
        Column<C>: TryFrom<Column<Any>>,
        C: ColumnType + std::hash::Hash,
    {
        self.inner
            .columns
            .iter()
            .filter_map(|c| (*c).try_into().ok())
            .collect()
    }
}

//impl<F: Copy + Default + std::fmt::Debug> From<RegionDataImpl<F>> for TableData<F> {
//    fn from(value: RegionDataImpl<F>) -> Self {
//        Self::new(value.shared.fixed)
//    }
//}

#[derive(Copy, Clone, Debug)]
pub struct RegionData<'a> {
    //shared: &'a SharedRegionData<F>,
    inner: &'a RegionDataImpl,
}

impl<'a> RegionData<'a> {
    pub fn new(inner: &'a RegionDataImpl) -> Self {
        Self { inner }
    }

    pub fn find_advice_name(&self, col: usize, row: usize) -> Cow<'a, FQN> {
        todo!()
        //self.shared
        //    .advice_names()
        //    .get(&(col, row))
        //    .map(Cow::Borrowed)
        //    .unwrap_or_else(|| {
        //        Cow::Owned(FQN::new(
        //            self.inner.name.as_str(),
        //            self.inner.index,
        //            &[],
        //            None,
        //        ))
        //    })
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

    pub fn find_fixed_col_assignment<F>(&self, col: Column<Fixed>, row: usize) -> Option<Value<F>>
    where
        F: Field,
    {
        todo!()
        //self.resolve_fixed(col.index(), row)
    }

    pub fn resolve_fixed<F>(&self, column: usize, row: usize) -> Option<Value<F>>
    where
        F: Field,
    {
        todo!()
        //self.shared.resolve_fixed(column, row)
    }

    pub fn enabled_selectors(&self) -> &'a HashMap<Selector, Vec<usize>> {
        self.inner.enabled_selectors()
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
}
