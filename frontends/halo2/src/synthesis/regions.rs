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
    ops::Range,
};

#[derive(Debug)]
pub struct RegionData<F> {
    /// The name of the region. Not required to be unique.
    name: String,
    /// The selectors that have been enabled in this region. All other selectors are by
    /// construction not enabled.
    enabled_selectors: HashMap<Selector, Vec<usize>>,
    /// The columns involved in this region.
    columns: HashSet<Column<Any>>,
    /// The rows that this region starts and ends on, if known.
    rows: Option<(usize, usize)>,
    /// Constant values assigned to fixed columns in the region.
    fixed: HashMap<(usize, usize), Value<F>>,
}

impl<F: Default + Clone> RegionData<F> {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            enabled_selectors: Default::default(),
            columns: Default::default(),
            rows: Default::default(),
            fixed: Default::default(),
        }
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

        // The region start is the earliest row assigned to.
        // The region end is the latest row assigned to.
        let (mut start, mut end) = self.rows.unwrap_or((row, row));
        if row < start {
            // The first row assigned was not at start 0 within the region.
            start = row;
        }
        if row > end {
            end = row;
        }
        self.rows = Some((start, end));
    }

    pub fn enable_selector(&mut self, s: Selector, row: usize) {
        self.enabled_selectors.entry(s).or_default().push(row);
    }

    pub fn assign_fixed<VR>(&mut self, fixed: Column<Fixed>, row: usize, value: Value<VR>)
    where
        F: Field,
        VR: Into<Assigned<F>>,
    {
        self.fixed
            .insert((fixed.index(), row), value.map(|vr| vr.into().evaluate()));
    }

    pub fn rows(&self) -> Range<usize> {
        self.rows.map(|(begin, end)| begin..end + 1).unwrap_or(0..0)
    }

    pub fn resolve_fixed(&self, column: &usize, row: &usize) -> Value<&F> {
        self.fixed
            .get(&(*column, *row))
            .and_then(|v| Some(v.as_ref()))
            .or_else(|| Some(Value::unknown()))
            .unwrap()
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

#[derive(Clone)]
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

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<ResolvedQuery<F>> {
        let r = self.resolve(self.advice_io, query.column_index(), query.rotation())?;
        // Advice cells go second so we need to step the value by the number of instance cells
        // that are of the same type (input or output)
        let r = self.step_advice_io(r);
        Ok(r.into())
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

#[derive(Clone)]
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
    pub fn gate_is_disabled(&self, selectors: &[&Selector]) -> bool {
        self.enabled()
            .is_disjoint(&selectors.iter().map(|s| *s).collect())
    }
}

impl<F: Field> QueryResolver<F> for RegionRow<'_, '_, F> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        let row = self.row.resolve_rotation(query.rotation())?;
        let value = self.region.resolve_fixed(&query.column_index(), &row);

        Ok(ResolvedQuery::Lit(value.copied()))
    }

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<ResolvedQuery<F>> {
        self.row.resolve_advice_query(query)
    }

    fn resolve_instance_query(&self, query: &InstanceQuery) -> Result<ResolvedQuery<F>> {
        self.row.resolve_instance_query(query)
    }
}

impl<F: Field> SelectorResolver for RegionRow<'_, '_, F> {
    fn resolve_selector(&self, selector: &Selector) -> Result<ResolvedSelector> {
        let selected = self
            .region
            .enabled_selectors
            .get(selector)
            .map(|rows| rows.contains(&self.row.row))
            .unwrap_or(false);
        Ok(ResolvedSelector::Const(selected.into()))
    }
}
