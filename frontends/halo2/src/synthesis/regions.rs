use crate::{
    backend::{
        func::{ArgNo, FieldId, FuncIO},
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    halo2::*,
    io::IOCell,
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

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FQN {
    region: String,
    region_idx: Option<RegionIndex>,
    namespaces: Vec<String>,
    tail: Option<String>,
}

impl fmt::Display for FQN {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn clean_string(s: &str) -> String {
            s.trim()
                .replace(|c: char| !c.is_ascii_alphanumeric() && c != '_', "_")
        }
        write!(f, "{}", clean_string(&self.region))?;
        if let Some(index) = self.region_idx {
            write!(f, "_{}", *index)?;
        }
        if !self.namespaces.is_empty() {
            write!(f, "__{}", clean_string(&self.namespaces.join("__")))?;
        }
        if let Some(name) = &self.tail {
            write!(f, "__{}", clean_string(name))?;
        }
        write!(f, "")
    }
}

impl FQN {
    pub fn new(
        region: &str,
        region_idx: Option<RegionIndex>,
        namespaces: &[String],
        tail: Option<String>,
    ) -> Self {
        Self {
            region: region.to_string(),
            region_idx,
            namespaces: namespaces.to_vec(),
            tail,
        }
    }
}

/// Data shared across regions.
#[derive(Debug, Default)]
pub struct SharedRegionData {
    advice_names: HashMap<(usize, usize), FQN>,
}

impl AddAssign for SharedRegionData {
    fn add_assign(&mut self, rhs: Self) {
        for (k, v) in rhs.advice_names {
            self.advice_names.insert(k, v);
        }
    }
}

type BlanketFills<F> = Vec<(RangeFrom<usize>, Value<F>)>;

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

#[derive(Debug)]
enum RegionKind {
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
            .advice_names
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

#[derive(Copy, Clone, Debug)]
pub struct RegionData<'a, F> {
    shared: &'a SharedRegionData,
    inner: &'a RegionDataImpl<F>,
}

impl<'a, F: Default + Clone + Copy + std::fmt::Debug> RegionData<'a, F> {
    pub fn find_advice_name(&self, col: usize, row: usize) -> Cow<'a, FQN> {
        self.shared
            .advice_names
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

#[derive(Default, Debug)]
pub struct Regions<F> {
    shared: SharedRegionData,
    regions: Vec<RegionDataImpl<F>>,
    tables: Vec<RegionDataImpl<F>>,
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
                        .index
                        .as_deref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "<unk>".to_owned()),
                    table.name
                );
                table.mark_as_table();
                self.tables.push(table);
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
                *region.index.unwrap(),
                region.name
            );
            region.mark_as_table();
            self.tables.push(region);
        } else {
            log::debug!(
                "Region {} {:?} added to the regions list",
                *region.index.unwrap(),
                region.name
            );
            self.shared += region.shared.take().unwrap();
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
            .map(|inner| RegionData {
                inner,
                shared: &self.shared,
            })
            //.chain(self.tables.iter().map(|inner| RegionData {
            //    inner,
            //    shared: &self.shared,
            //}))
            .collect()
    }

    pub fn mark_current_as_table(&mut self) {
        self.current_is_table = true;
    }

    pub fn seen_advice_cells(&self) -> impl Iterator<Item = (&(usize, usize), &FQN)> {
        self.shared.advice_names.iter()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Row<'io, F> {
    row: usize,
    advice_io: &'io CircuitIO<Advice>,
    instance_io: &'io CircuitIO<Instance>,
    _marker: PhantomData<F>,
}

impl<'io, F> Row<'io, F> {
    pub fn new(
        row: usize,
        advice_io: &'io CircuitIO<Advice>,
        instance_io: &'io CircuitIO<Instance>,
    ) -> Self {
        Self {
            row,
            advice_io,
            instance_io,
            _marker: Default::default(),
        }
    }

    fn resolve_rotation(&self, rot: Rotation) -> Result<usize> {
        let row: isize = self.row.try_into()?;
        let rot: isize = rot.0.try_into()?;
        if -rot > row {
            bail!("Row underflow");
        }
        Ok((row + rot).try_into()?)
    }

    fn resolve_as<O: From<usize> + Into<FuncIO>, C: ColumnType>(
        &self,
        io: &[IOCell<C>],
        col: usize,
        rot: Rotation,
    ) -> Result<Option<FuncIO>> {
        let target_cell = (col, self.resolve_rotation(rot)?);
        Ok(io
            .iter()
            .map(|(col, row)| (col.index(), *row))
            .enumerate()
            .find_map(|(idx, cell)| {
                if cell == target_cell {
                    Some(O::from(idx))
                } else {
                    None
                }
            })
            .map(Into::into))
    }

    fn resolve<C: ColumnType>(
        &self,
        io: &CircuitIO<C>,
        col: usize,
        rot: Rotation,
    ) -> Result<FuncIO> {
        let as_input = self.resolve_as::<ArgNo, C>(io.inputs(), col, rot)?;
        let as_output = self.resolve_as::<FieldId, C>(io.outputs(), col, rot)?;

        Ok(match (as_input, as_output) {
            (None, None) => FuncIO::Advice(col, self.resolve_rotation(rot)?),
            (None, Some(r)) => r,
            (Some(r), None) => r,
            (Some(_), Some(_)) => bail!("Query is both an input and an output in main function"),
        })
    }

    fn step_advice_io(&self, io: FuncIO) -> FuncIO {
        match io {
            FuncIO::Arg(arg_no) => (arg_no.offset_by(self.instance_io.inputs().len())).into(),
            FuncIO::Field(field_id) => {
                (field_id.offset_by(self.instance_io.outputs().len())).into()
            }
            io => io,
        }
    }
}

impl<F: Field> QueryResolver<F> for Row<'_, F> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        let row = self.resolve_rotation(query.rotation())?;
        Ok(ResolvedQuery::IO(FuncIO::Fixed(query.column_index(), row)))
    }

    fn resolve_advice_query<'a>(
        &'a self,
        query: &AdviceQuery,
    ) -> Result<(ResolvedQuery<F>, Option<Cow<'a, FQN>>)> {
        let r = self.resolve(self.advice_io, query.column_index(), query.rotation())?;
        // Advice cells go second so we need to step the value by the number of instance cells
        // that are of the same type (input or output)
        let r = self.step_advice_io(r);
        Ok((r.into(), None))
    }

    fn resolve_instance_query(&self, query: &InstanceQuery) -> Result<ResolvedQuery<F>> {
        let r = self.resolve(self.instance_io, query.column_index(), query.rotation())?;
        Ok(r.into())
    }
}

impl<F> SelectorResolver for Row<'_, F> {
    fn resolve_selector(&self, _selector: &Selector) -> Result<ResolvedSelector> {
        unreachable!()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RegionRow<'r, 'io, F: Field> {
    region: RegionData<'r, F>,
    row: Row<'io, F>,
}

impl<'r, 'io, F: Field> RegionRow<'r, 'io, F> {
    pub fn new(
        region: RegionData<'r, F>,
        row: usize,
        advice_io: &'io CircuitIO<Advice>,
        instance_io: &'io CircuitIO<Instance>,
    ) -> Self {
        Self {
            region,
            row: Row::new(row, advice_io, instance_io),
        }
    }

    fn enabled(&self) -> HashSet<&'r Selector> {
        self.region
            .inner
            .selectors_enabled_for_row(self.row.row)
            .into_iter()
            .collect()
    }

    #[inline]
    pub fn gate_is_disabled(&self, selectors: &HashSet<&Selector>) -> bool {
        self.enabled().is_disjoint(selectors)
    }

    #[inline]
    pub fn region_name(&self) -> &'r str {
        &self.region.inner.name
    }

    #[inline]
    pub fn region_index(&self) -> Option<usize> {
        self.region.inner.index.map(|f| *f)
    }

    #[inline]
    pub fn row_number(&self) -> usize {
        self.row.row
    }

    #[inline]
    pub fn header(&self) -> String {
        match (&self.region.inner.kind, &self.region.inner.index) {
            (RegionKind::Region, None) => format!("region <unk> {:?}", self.region.inner.name),
            (RegionKind::Region, Some(index)) => {
                format!("region {} {:?}", **index, self.region.inner.name)
            }
            (RegionKind::Table, None) => format!("table {:?}", self.region.inner.name),
            _ => unreachable!(),
        }
    }
}

impl<F: Field> QueryResolver<F> for RegionRow<'_, '_, F> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        let row = self.row.resolve_rotation(query.rotation())?;

        Ok(
            match self.region.inner.resolve_fixed(query.column_index(), row) {
                Some(v) => v.try_into()?,
                None => ResolvedQuery::IO(FuncIO::Fixed(query.column_index(), row)),
            },
        )
    }

    fn resolve_advice_query<'a>(
        &'a self,
        query: &AdviceQuery,
    ) -> Result<(ResolvedQuery<F>, Option<Cow<'a, FQN>>)> {
        let (r, _): (ResolvedQuery<F>, _) = self.row.resolve_advice_query(query)?;

        match r {
            l @ ResolvedQuery::Lit(_) => Ok((l, None)),
            io @ ResolvedQuery::IO(func_io) => Ok((
                io,
                Some(match func_io {
                    FuncIO::Advice(col, row) => self.region.find_advice_name(col, row),
                    _ => Cow::Owned(FQN::new(
                        &self.region.inner.name,
                        self.region.inner.index,
                        &[],
                        None,
                    )),
                }),
            )),
        }
    }

    fn resolve_instance_query(&self, query: &InstanceQuery) -> Result<ResolvedQuery<F>> {
        self.row.resolve_instance_query(query)
    }
}

impl<F: Field> SelectorResolver for RegionRow<'_, '_, F> {
    fn resolve_selector(&self, selector: &Selector) -> Result<ResolvedSelector> {
        let selected = self
            .region
            .inner
            .inner
            .enabled_selectors
            .get(selector)
            .map(|rows| rows.contains(&self.row.row))
            .unwrap_or(false);
        Ok(ResolvedSelector::Const(selected.into()))
    }
}
