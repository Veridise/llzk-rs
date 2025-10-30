use super::{RegionData, Row};
use crate::{
    backend::func::FuncIO,
    gates::SelectorSet,
    halo2::*,
    io::{AdviceIO, InstanceIO},
    resolvers::{
        FixedQueryResolver, QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver,
    },
};
use anyhow::Result;
use std::borrow::Cow;

#[derive(Copy, Clone, Debug)]
pub struct RegionRow<'r, 'io, 'fq, F: Field> {
    region: RegionData<'r>,
    row: Row<'io, 'fq, F>,
}

impl<'r, 'io, 'fq, F: Field> RegionRow<'r, 'io, 'fq, F> {
    pub fn new(
        region: RegionData<'r>,
        row: usize,
        advice_io: &'io AdviceIO,
        instance_io: &'io InstanceIO,
        fqr: &'fq dyn FixedQueryResolver<F>,
    ) -> Self {
        Self {
            region,
            row: Row::new(row, advice_io, instance_io, fqr),
        }
    }

    /// Changes the priority to inputs.
    pub fn prioritize_inputs(self) -> Self {
        Self {
            region: self.region,
            row: self.row.prioritize_inputs(),
        }
    }

    /// Changes the priority to outputs.
    pub fn prioritize_outputs(self) -> Self {
        Self {
            region: self.region,
            row: self.row.prioritize_outputs(),
        }
    }

    fn enabled(&self) -> Cow<SelectorSet> {
        self.region.selectors_enabled_for_row(self.row.row)
    }

    //pub fn region_index(&self) -> Option<usize> {
    //    self.region.index().map(|f| *f)
    //}

    //pub fn region_name(&self) -> &str {
    //    self.region.name()
    //}

    pub fn row_number(&self) -> usize {
        self.row.row
    }

    //pub fn region_index_as_str(&self) -> String {
    //    match self.region_index() {
    //        Some(i) => i.to_string(),
    //        None => "<unk>".to_string(),
    //    }
    //}

    #[inline]
    pub fn gate_is_disabled(&self, selectors: &SelectorSet) -> bool {
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

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<ResolvedQuery<F>> {
        log::debug!("Resolving query: {query:?}");
        let base = self
            .region
            .start()
            .ok_or_else(|| anyhow::anyhow!("Region does not have a size"))?;
        self.row
            .resolve_advice_query_impl(query, |col, row| match self.region.relativize(row) {
                Some(row) => FuncIO::advice_rel(col, base, row),
                None => FuncIO::advice_abs(col, row),
            })
            .map(ResolvedQuery::IO)
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
            .get(&self.row_number())
            .is_some_and(|selectors| selectors.contains(selector.index()));
        Ok(ResolvedSelector::Const(selected.into()))
    }
}
