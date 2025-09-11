use super::{RegionData, Row, FQN};
use crate::{
    backend::{
        func::FuncIO,
        resolvers::{
            FixedQueryResolver, QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver,
        },
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

/// The kind of region [`Row`] is using.
///
/// Used by [`RegionRow`] for resolving Advice queries
#[derive(Copy, Clone)]
pub enum RowMode {
    /// The row is absolute to the circuit.
    Absolute,
    /// The row is relative to the first row of the circuit.
    Relative,
}

impl Default for RowMode {
    fn default() -> Self {
        RowMode::Absolute
    }
}

#[derive(Copy, Clone)]
pub struct RegionRow<'r, 'io, 'fq, F: Field> {
    region: RegionData<'r>,
    row: Row<'io, 'fq, F>,
    mode: RowMode,
}

impl<'r, 'io, 'fq, F: Field> RegionRowLike for RegionRow<'r, 'io, 'fq, F> {
    fn region_index(&self) -> Option<usize> {
        self.region.index().map(|f| *f)
    }

    fn region_name(&self) -> &str {
        self.region.name()
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
        Self::new_with_mode(region, row, Default::default(), advice_io, instance_io, fqr)
    }

    pub fn new_with_mode(
        region: RegionData<'r>,
        row: usize,
        mode: RowMode,
        advice_io: &'io CircuitIO<Advice>,
        instance_io: &'io CircuitIO<Instance>,
        fqr: &'fq dyn FixedQueryResolver<F>,
    ) -> Self {
        Self {
            region,
            row: Row::new(row, advice_io, instance_io, fqr),
            mode,
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
        log::debug!("Resolving query: {query:?}");
        let base = self
            .region
            .start()
            .ok_or_else(|| anyhow::anyhow!("Region does not have a size"))?;
        self.row
            .resolve_advice_query_impl(query, |col, row| match self.mode {
                RowMode::Absolute => match self.region.relativize(row) {
                    Some(row) => FuncIO::advice_rel(col, base, row * 10),
                    None => FuncIO::advice_abs(col, row),
                },

                RowMode::Relative => {
                    log::debug!("region extent: {:?}, ", self.region.rows());
                    debug_assert!(
                        isize::try_from(base).unwrap() + query.rotation().0 as isize
                            <= isize::try_from(base).unwrap()
                                + query.rotation().0 as isize
                                + isize::try_from(row).unwrap(),
                        "Assertion failed: {base}+{} <= {base}+{}+{row}",
                        query.rotation().0,
                        query.rotation().0
                    );
                    FuncIO::advice_rel(col, base, row)
                }
            })
            .map(ResolvedQuery::IO)
            .map(|rq| (rq, None))
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
