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
use data::{RegionDataImpl, RegionKind};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
    marker::PhantomData,
    ops::{AddAssign, Range, RangeFrom},
};

mod data;
mod fqn;
mod shared;
mod table;

pub use data::RegionData;
pub use fqn::FQN;
pub use shared::SharedRegionData;
pub use table::TableData;

type BlanketFills<F> = Vec<(RangeFrom<usize>, Value<F>)>;

#[derive(Default, Debug)]
pub struct Regions<F: Copy> {
    shared: SharedRegionData,
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
            self.shared += region.take_shared().unwrap();
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

pub trait RegionRowLike {
    fn region_index(&self) -> Option<usize>;

    fn region_name(&self) -> &str;

    fn row_number(&self) -> usize;
}

#[derive(Copy, Clone, Debug)]
pub struct RegionRow<'r, 'io, F: Field> {
    region: RegionData<'r, F>,
    row: Row<'io, F>,
}

impl<'r, 'io, F: Field> RegionRowLike for RegionRow<'r, 'io, F> {
    fn region_index(&self) -> Option<usize> {
        self.region.inner.index().map(|f| *f)
    }

    fn region_name(&self) -> &str {
        &self.region.inner.name()
    }

    fn row_number(&self) -> usize {
        self.row.row
    }
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
    pub fn header(&self) -> String {
        match (&self.region.inner.kind(), &self.region.inner.index()) {
            (RegionKind::Region, None) => format!("region <unk> {:?}", self.region.inner.name()),
            (RegionKind::Region, Some(index)) => {
                format!("region {} {:?}", **index, self.region.inner.name())
            }
            (RegionKind::Table, None) => format!("table {:?}", self.region.inner.name()),
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
                        &self.region.inner.name(),
                        self.region.inner.index(),
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
            .enabled_selectors()
            .get(selector)
            .map(|rows| rows.contains(&self.row.row))
            .unwrap_or(false);
        Ok(ResolvedSelector::Const(selected.into()))
    }
}
