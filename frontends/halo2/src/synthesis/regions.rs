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
    collections::{HashMap, HashSet},
    fmt,
    ops::Range,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FQN {
    region: String,
    namespaces: Vec<String>,
    tail: Option<String>,
}

impl fmt::Display for FQN {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.region)?;
        if !self.namespaces.is_empty() {
            write!(f, " :: {}", self.namespaces.join(" :: "))?;
        }
        if let Some(name) = &self.tail {
            write!(f, " :: {name}")?;
        }
        write!(f, "")
    }
}

impl FQN {
    pub fn new(region: &str, namespaces: &[String], tail: Option<String>) -> Self {
        Self {
            region: region.to_string(),
            namespaces: namespaces.to_vec(),
            tail,
        }
    }
}

#[derive(Debug, Default)]
struct RegionDataInner<F> {
    /// The selectors that have been enabled in this region. All other selectors are by
    /// construction not enabled.
    enabled_selectors: HashMap<Selector, Vec<usize>>,
    /// The columns involved in this region.
    columns: HashSet<Column<Any>>,
    /// The rows that this region starts and ends on, if known.
    rows: Option<(usize, usize)>,
    /// Constant values assigned to fixed columns in the region.
    fixed: HashMap<(usize, usize), Value<F>>,
    advice_names: HashMap<(usize, usize), FQN>,
    advice_columns: HashSet<Column<Advice>>,
    namespaces: Vec<String>,
}

#[derive(Debug)]
pub struct RegionData<F> {
    #[allow(dead_code)]
    /// The name of the region. Not required to be unique.
    name: String,
    inner: RegionDataInner<F>,
}

impl<F: Default + Clone> RegionData<F> {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            inner: Default::default(),
        }
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

        if let Any::Advice(_) = column.column_type() {
            self.inner.advice_columns.insert(column.try_into().unwrap());
            self.allocate_advice_names();
        }
    }

    /// Creates anonymous advice cells in the cells that are within the confines of the region.
    /// If in a later stage the cell has a proper name it will overwrite this anonymous name.
    fn allocate_advice_names(&mut self) {
        if let Some((start, end)) = self.inner.rows {
            for row in start..=end {
                for col in &self.inner.advice_columns {
                    if self.inner.advice_names.contains_key(&(col.index(), row)) {
                        continue;
                    }
                    let anon_fqn = FQN::new(self.name.as_str(), &self.inner.namespaces, None);
                    self.inner.advice_names.insert((col.index(), row), anon_fqn);
                }
            }
        }
    }

    pub fn enable_selector(&mut self, s: Selector, row: usize) {
        self.inner.enabled_selectors.entry(s).or_default().push(row);
    }

    pub fn assign_fixed<VR>(&mut self, fixed: Column<Fixed>, row: usize, value: Value<VR>)
    where
        F: Field,
        VR: Into<Assigned<F>>,
    {
        self.inner
            .fixed
            .insert((fixed.index(), row), value.map(|vr| vr.into().evaluate()));
    }

    pub fn rows(&self) -> Range<usize> {
        self.inner
            .rows
            .map(|(begin, end)| begin..end + 1)
            .unwrap_or(0..0)
    }

    pub fn resolve_fixed(&self, column: &usize, row: &usize) -> Value<&F> {
        self.inner
            .fixed
            .get(&(*column, *row))
            .and_then(|v| Some(v.as_ref()))
            .or_else(|| Some(Value::unknown()))
            .unwrap()
    }

    pub fn note_advice(&mut self, column: Column<Advice>, row: usize, name: String) {
        let fqn = FQN::new(self.name.as_str(), &self.inner.namespaces, name.into());
        self.inner.advice_names.insert((column.index(), row), fqn);
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

    pub fn find_advice_name(&self, col: usize, row: usize) -> FQN {
        self.inner
            .advice_names
            .get(&(col, row))
            .cloned()
            .unwrap_or(FQN::new(&self.name, &[], None))
    }
}

#[derive(Default, Debug)]
pub struct Regions<F> {
    regions: Vec<RegionData<F>>,
    current: Option<RegionData<F>>,
}

impl<F: Default + Clone> Regions<F> {
    pub fn push<NR, N>(&mut self, region_name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        assert!(self.current.is_none());
        self.current = Some(RegionData::new(region_name()));
    }

    pub fn commit(&mut self) {
        self.regions.push(self.current.take().unwrap());
    }

    pub fn edit<FN, FR>(&mut self, f: FN) -> Option<FR>
    where
        FN: FnOnce(&mut RegionData<F>) -> FR,
    {
        if let Some(region) = self.current.as_mut() {
            return Some(f(region));
        }
        None
    }

    pub fn regions(&self) -> &[RegionData<F>] {
        &self.regions
    }
}

#[derive(Copy, Clone)]
pub struct Row<'io> {
    row: usize,
    advice_io: &'io CircuitIO<Advice>,
    instance_io: &'io CircuitIO<Instance>,
}

impl<'io> Row<'io> {
    pub fn new(
        row: usize,
        advice_io: &'io CircuitIO<Advice>,
        instance_io: &'io CircuitIO<Instance>,
    ) -> Self {
        Self {
            row,
            advice_io,
            instance_io,
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
            (None, None) => FuncIO::Temp(col, self.resolve_rotation(rot)?),
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

impl<F: Field> QueryResolver<F> for Row<'_> {
    fn resolve_fixed_query(&self, _query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        unreachable!()
    }

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<(ResolvedQuery<F>, Option<FQN>)> {
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

impl SelectorResolver for Row<'_> {
    fn resolve_selector(&self, _selector: &Selector) -> Result<ResolvedSelector> {
        unreachable!()
    }
}

#[derive(Copy, Clone)]
pub struct RegionRow<'r, 'io, F: Field> {
    region: &'r RegionData<F>,
    row: Row<'io>,
}

impl<'r, 'io, F: Field> RegionRow<'r, 'io, F> {
    pub fn new(
        region: &'r RegionData<F>,
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
            .selectors_enabled_for_row(self.row.row)
            .into_iter()
            .collect()
    }

    #[inline]
    pub fn gate_is_disabled(&self, selectors: &HashSet<&Selector>) -> bool {
        self.enabled().is_disjoint(selectors)
    }
}

impl<F: Field> QueryResolver<F> for RegionRow<'_, '_, F> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        let row = self.row.resolve_rotation(query.rotation())?;
        let value = self.region.resolve_fixed(&query.column_index(), &row);

        Ok(ResolvedQuery::Lit(value.copied()))
    }

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<(ResolvedQuery<F>, Option<FQN>)> {
        let (r, _): (ResolvedQuery<F>, _) = self.row.resolve_advice_query(query)?;

        match r {
            l @ ResolvedQuery::Lit(_) => Ok((l, None)),
            io @ ResolvedQuery::IO(func_io) => Ok((
                io,
                Some(match func_io {
                    FuncIO::Temp(col, row) => self.region.find_advice_name(col, row),
                    _ => FQN::new(&self.region.name, &[], None),
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
            .enabled_selectors
            .get(selector)
            .map(|rows| rows.contains(&self.row.row))
            .unwrap_or(false);
        Ok(ResolvedSelector::Const(selected.into()))
    }
}
