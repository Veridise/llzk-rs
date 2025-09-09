use super::{RegionData, Row, FQN};
use crate::{
    backend::resolvers::{
        FixedQueryResolver, QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver,
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

#[derive(Copy, Clone)]
pub struct RegionRow<'r, 'io, 'fq, F: Field> {
    region: RegionData<'r>,
    row: Row<'io, 'fq, F>,
}

impl<'r, 'io, 'fq, F: Field> RegionRowLike for RegionRow<'r, 'io, 'fq, F> {
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

impl<'r, 'io, 'fq, F: Field> RegionRow<'r, 'io, 'fq, F> {
    pub fn new(
        region: RegionData<'r>,
        row: usize,
        advice_io: &'io CircuitIO<Advice>,
        instance_io: &'io CircuitIO<Instance>,
        fqr: &'fq dyn FixedQueryResolver<F>,
    ) -> Self {
        Self {
            region,
            row: Row::new(row, advice_io, instance_io, fqr),
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

impl<F: Field> QueryResolver<F> for RegionRow<'_, '_, '_, F> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        let row = self.row.resolve_rotation(query.rotation())?;
        self.row
            .fqr
            .resolve_query(query, row)
            .map(ResolvedQuery::Lit)
    }

    fn resolve_advice_query<'a>(
        &'a self,
        query: &AdviceQuery,
    ) -> Result<(ResolvedQuery<F>, Option<Cow<'a, FQN>>)> {
        let (r, _): (ResolvedQuery<F>, _) = self.row.resolve_advice_query(query)?;

        match r {
            l @ ResolvedQuery::Lit(_) => Ok((l, None)),
            io @ ResolvedQuery::IO(_func_io) => Ok((io, None)),
        }
    }

    fn resolve_instance_query(&self, query: &InstanceQuery) -> Result<ResolvedQuery<F>> {
        self.row.resolve_instance_query(query)
    }
}

impl<F: Field> SelectorResolver for RegionRow<'_, '_, '_, F> {
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
