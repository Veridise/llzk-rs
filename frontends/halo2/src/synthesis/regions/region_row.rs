use super::{RegionData, Row, FQN};
use crate::{
    backend::{
        func::FuncIO,
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    halo2::*,
    CircuitIO,
};
use anyhow::Result;
use std::{borrow::Cow, collections::HashSet};

pub trait RegionRowLike {
    fn region_index(&self) -> Option<usize>;

    fn region_index_as_str(&self) -> String {
        match self.region_index() {
            Some(i) => i.to_string(),
            None => "<unk>".to_string(),
        }
    }

    fn region_name(&self) -> &str;

    fn row_number(&self) -> usize;

    fn header(&self) -> String {
        format!(
            "region {} '{}' @ row {}",
            self.region_index_as_str(),
            self.region_name(),
            self.row_number()
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RegionRow<'r, 'io> {
    region: RegionData<'r>,
    row: Row<'io>,
}

impl<'r, 'io> RegionRowLike for RegionRow<'r, 'io> {
    fn region_index(&self) -> Option<usize> {
        self.region.index().map(|f| *f)
    }

    fn region_name(&self) -> &str {
        &self.region.name()
    }

    fn row_number(&self) -> usize {
        self.row.row
    }
}

impl<'r, 'io> RegionRow<'r, 'io> {
    pub fn new(
        region: RegionData<'r>,
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

    #[inline]
    pub fn header(&self) -> String {
        self.region.header()
    }
}

impl<F: Field> QueryResolver<F> for RegionRow<'_, '_> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        let row = self.row.resolve_rotation(query.rotation())?;

        Ok(match self.region.resolve_fixed(query.column_index(), row) {
            Some(v) => v.try_into()?,
            None => ResolvedQuery::IO(FuncIO::fixed_abs(query.column_index(), row)),
        })
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
                    FuncIO::Advice(adv) => self.region.find_advice_name(adv.col(), adv.row()),
                    _ => Cow::Owned(FQN::new(
                        &self.region.name(),
                        self.region.index(),
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

impl SelectorResolver for RegionRow<'_, '_> {
    fn resolve_selector(&self, selector: &Selector) -> Result<ResolvedSelector> {
        let selected = self
            .region
            .enabled_selectors()
            .get(selector)
            .map(|rows| rows.contains(&self.row.row))
            .unwrap_or(false);
        Ok(ResolvedSelector::Const(selected.into()))
    }
}
