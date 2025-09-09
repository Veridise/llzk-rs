use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Range;

use crate::expressions::ScopedExpression;
use crate::ir::stmt::IRStmt;
use crate::synthesis::regions::{RegionData, RegionRow};
use crate::{halo2::*, CircuitIO};

pub enum RewriteError {
    NoMatch,
    Err(anyhow::Error),
}

/// Scope in which a gate is being called
#[derive(Copy, Clone, Debug)]
pub struct GateScope<'a, F>
where
    F: Field,
{
    gate: &'a Gate<F>,
    region: RegionData<'a>,
    /// The bounds are [start,end).
    row_bounds: (usize, usize),
    advice_io: &'a CircuitIO<Advice>,
    instance_io: &'a CircuitIO<Instance>,
}

impl<'a, F: Field> GateScope<'a, F> {
    pub(crate) fn new(
        gate: &'a Gate<F>,
        region: RegionData<'a>,
        row_bounds: (usize, usize),
        advice_io: &'a CircuitIO<Advice>,
        instance_io: &'a CircuitIO<Instance>,
    ) -> Self {
        Self {
            gate,
            region,
            row_bounds,
            advice_io,
            instance_io,
        }
    }

    pub(crate) fn region(&self) -> RegionData<'a> {
        self.region
    }

    pub(crate) fn region_row(&self, row: usize) -> anyhow::Result<RegionRow<'a, 'a>> {
        if !self.rows().contains(&row) {
            anyhow::bail!(
                "Row {} is not within the rows of the scope [{}, {}]",
                row,
                self.start_row(),
                self.end_row()
            )
        }
        Ok(RegionRow::new(
            self.region(),
            row,
            self.advice_io,
            self.instance_io,
        ))
    }

    pub(crate) fn region_rows(&self) -> impl Iterator<Item = RegionRow<'a, 'a>> {
        self.rows()
            .map(|row| RegionRow::new(self.region(), row, self.advice_io, self.instance_io))
    }

    pub fn gate_name(&self) -> &str {
        self.gate.name()
    }

    pub fn polynomials(&self) -> &'a [Expression<F>] {
        self.gate.polynomials()
    }

    pub fn polynomials_per_row(
        &self,
    ) -> anyhow::Result<Vec<(&'a Expression<F>, Vec<(usize, Expression<F>)>)>> {
        self.polynomials()
            .into_iter()
            .map(|e| {
                let rows = self
                    .rows()
                    .map(|row| {
                        let folded = self.fold_polynomial_in_row(e, row)?;
                        Ok((row, folded))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Ok((e, rows))
            })
            .collect()
    }

    fn fold_polynomial_in_row(
        &self,
        e: &Expression<F>,
        row: usize,
    ) -> anyhow::Result<Expression<F>> {
        let region_row = self.region_row(row)?;
        let scoped = ScopedExpression::from_ref(e, region_row);
        Ok(scoped.fold_constants())
    }

    pub fn region_name(&self) -> &str {
        self.region.name()
    }

    pub fn region_index(&self) -> Option<RegionIndex> {
        self.region.index()
    }

    pub fn region_header(&self) -> impl ToString {
        self.region.header()
    }

    pub fn start_row(&self) -> usize {
        self.row_bounds.0
    }

    /// The last row of the region.
    pub fn end_row(&self) -> usize {
        let end = self.row_bounds.1;
        if end == 0 {
            return end;
        }
        end - 1
    }

    pub fn rows(&self) -> Range<usize> {
        (self.row_bounds.0)..(self.row_bounds.1)
    }
}

/// The type used for rewriting the gates. Each expression has an associated row that is used as
/// the base offset on the queries.
pub type RewriteOutput<'a, F> = IRStmt<(usize, Cow<'a, Expression<F>>)>;

pub trait GateRewritePattern<F> {
    fn match_gate<'a>(&self, gate: GateScope<'a, F>) -> Result<(), RewriteError>
    where
        F: Field,
    {
        panic!("Implement match_gate and rewrite_gate OR match_and_rewrite")
    }

    fn rewrite_gate<'a>(
        &self,
        gate: GateScope<'a, F>,
    ) -> Result<RewriteOutput<'a, F>, anyhow::Error>
    where
        F: Field,
    {
        panic!("Implement match_gate and rewrite_gate OR match_and_rewrite")
    }

    fn match_and_rewrite<'a>(
        &self,
        gate: GateScope<'a, F>,
    ) -> Result<RewriteOutput<'a, F>, RewriteError>
    where
        F: Field,
    {
        self.match_gate(gate)?;
        self.rewrite_gate(gate).map_err(RewriteError::Err)
    }
}

pub trait GateCallbacks<F> {
    /// Asks wether a gate's polynomial whose selectors are all disabled for a given region should be emitted or
    /// not. Defaults to true.
    fn ignore_disabled_gates(&self) -> bool {
        true
    }

    /// Asks for a list of patterns that are checked before the default ones.
    fn patterns(&self) -> Vec<Box<dyn GateRewritePattern<F>>>
    where
        F: Field;
}

pub(crate) struct DefaultGateCallbacks;

impl<F> GateCallbacks<F> for DefaultGateCallbacks {
    fn patterns(&self) -> Vec<Box<dyn GateRewritePattern<F>>>
    where
        F: Field,
    {
        vec![]
    }
}

#[derive(Default)]
pub(crate) struct RewritePatternSet<F>(Vec<Box<dyn GateRewritePattern<F>>>);

impl<F> RewritePatternSet<F> {
    pub fn add(&mut self, p: impl GateRewritePattern<F> + 'static) {
        self.0.push(Box::new(p))
    }
}

impl<F> Extend<Box<dyn GateRewritePattern<F>>> for RewritePatternSet<F> {
    fn extend<T: IntoIterator<Item = Box<dyn GateRewritePattern<F>>>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

pub(crate) struct RewritePatternSetIter<'a, F>(
    std::slice::Iter<'a, Box<dyn GateRewritePattern<F>>>,
);

impl<F> GateRewritePattern<F> for RewritePatternSet<F> {
    fn match_and_rewrite<'a>(
        &self,
        gate: GateScope<'a, F>,
    ) -> Result<RewriteOutput<'a, F>, RewriteError>
    where
        F: Field,
    {
        let mut errors = vec![];
        log::debug!(
            "Starting match for gate '{}' on region '{}'",
            gate.gate_name(),
            gate.region_name()
        );
        for pattern in self.0.iter() {
            log::debug!("Starting pattern");
            match pattern.match_and_rewrite(gate) {
                Ok(r) => {
                    log::debug!("Returning a value from the pattern");
                    return Ok(r);
                }
                Err(RewriteError::NoMatch) => {
                    log::debug!("Pattern did not match");
                }
                Err(RewriteError::Err(e)) => {
                    log::debug!("Pattern generated an error: {e}");
                    errors.push(e);
                }
            }
        }

        Err(if errors.is_empty() {
            log::debug!("No errors so returning NoMatch");
            RewriteError::NoMatch
        } else {
            log::debug!("Returning {} errors", errors.len());
            RewriteError::Err(anyhow::anyhow!(errors
                .into_iter()
                .flat_map(|e: anyhow::Error| [e.to_string(), "\n".to_string()])
                .collect::<String>()))
        })
    }
}

fn find_in_binop<'a, QR, F, Q>(lhs: &'a Expression<F>, rhs: &'a Expression<F>, q: Q) -> HashSet<QR>
where
    F: Field,
    Q: Fn(&'a Expression<F>) -> HashSet<QR>,
    QR: Hash + Eq + Clone,
{
    q(lhs).union(&q(rhs)).cloned().collect()
}

pub(crate) fn find_selectors<F: Field>(poly: &Expression<F>) -> HashSet<&Selector> {
    match poly {
        Expression::Selector(selector) => [selector].into(),
        Expression::Negated(expression) => find_selectors(expression),
        Expression::Sum(lhs, rhs) => find_in_binop(lhs, rhs, find_selectors),
        Expression::Product(lhs, rhs) => find_in_binop(lhs, rhs, find_selectors),
        Expression::Scaled(expression, _) => find_selectors(expression),
        _ => Default::default(),
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AnyQuery {
    Advice(AdviceQuery),
    Instance(InstanceQuery),
    Fixed(FixedQuery),
}

impl AnyQuery {
    pub fn expr<F>(&self) -> Expression<F> {
        match self {
            AnyQuery::Advice(advice_query) => Expression::Advice(*advice_query),
            AnyQuery::Instance(instance_query) => Expression::Instance(*instance_query),
            AnyQuery::Fixed(fixed_query) => Expression::Fixed(*fixed_query),
        }
    }
    pub fn column_index(&self) -> usize {
        match self {
            AnyQuery::Advice(advice_query) => advice_query.column_index(),
            AnyQuery::Instance(instance_query) => instance_query.column_index(),
            AnyQuery::Fixed(fixed_query) => fixed_query.column_index(),
        }
    }

    pub fn rotation(&self) -> Rotation {
        match self {
            AnyQuery::Advice(advice_query) => advice_query.rotation(),
            AnyQuery::Instance(instance_query) => instance_query.rotation(),
            AnyQuery::Fixed(fixed_query) => fixed_query.rotation(),
        }
    }

    pub fn phase(&self) -> Option<u8> {
        match self {
            AnyQuery::Advice(advice_query) => Some(advice_query.phase()),
            _ => None,
        }
    }

    pub fn type_id(&self) -> u8 {
        match self {
            AnyQuery::Advice(_) => 0,
            AnyQuery::Instance(_) => 1,
            AnyQuery::Fixed(_) => 2,
        }
    }

    pub fn to_tuple(&self) -> (u8, usize, i32, Option<u8>) {
        (
            self.type_id(),
            self.column_index(),
            self.rotation().0,
            self.phase(),
        )
    }
}

impl PartialEq<FixedQuery> for AnyQuery {
    fn eq(&self, other: &FixedQuery) -> bool {
        match self {
            Self::Fixed(query) => query == other,
            _ => false,
        }
    }
}

impl PartialEq<InstanceQuery> for AnyQuery {
    fn eq(&self, other: &InstanceQuery) -> bool {
        match self {
            Self::Instance(query) => query == other,
            _ => false,
        }
    }
}

impl PartialEq<AdviceQuery> for AnyQuery {
    fn eq(&self, other: &AdviceQuery) -> bool {
        match self {
            Self::Advice(query) => query == other,
            _ => false,
        }
    }
}

impl PartialOrd for AnyQuery {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AnyQuery {
    /// Column metadata is ordered lexicographically by column type, then index, and lastly
    /// rotation. In the case of Advice columns the last element is the phase.
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_tuple().cmp(&other.to_tuple())
    }
}

impl Hash for AnyQuery {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let (typ, col, rot, pha) = self.to_tuple();
        typ.hash(state);
        col.hash(state);
        rot.hash(state);
        pha.hash(state);
    }
}

impl From<&AdviceQuery> for AnyQuery {
    fn from(query: &AdviceQuery) -> Self {
        Self::Advice(*query)
    }
}

impl From<&InstanceQuery> for AnyQuery {
    fn from(query: &InstanceQuery) -> Self {
        Self::Instance(*query)
    }
}

impl From<&FixedQuery> for AnyQuery {
    fn from(query: &FixedQuery) -> Self {
        Self::Fixed(*query)
    }
}

impl From<AdviceQuery> for AnyQuery {
    fn from(query: AdviceQuery) -> Self {
        Self::Advice(query)
    }
}

impl From<InstanceQuery> for AnyQuery {
    fn from(query: InstanceQuery) -> Self {
        Self::Instance(query)
    }
}

impl From<FixedQuery> for AnyQuery {
    fn from(query: FixedQuery) -> Self {
        Self::Fixed(query)
    }
}

fn find_queries<F: Field>(poly: &Expression<F>) -> HashSet<AnyQuery> {
    match poly {
        Expression::Advice(query) => [query.into()].into(),
        Expression::Instance(query) => [query.into()].into(),
        Expression::Fixed(query) => [query.into()].into(),
        Expression::Negated(expression) => find_queries(expression),
        Expression::Sum(lhs, rhs) => find_in_binop(lhs, rhs, find_queries),
        Expression::Product(lhs, rhs) => find_in_binop(lhs, rhs, find_queries),
        Expression::Scaled(expression, _) => find_queries(expression),
        _ => Default::default(),
    }
}

pub type GateArity<'a> = (Vec<&'a Selector>, Vec<AnyQuery>);

pub fn find_gate_selector_set<F: Field>(constraints: &[Expression<F>]) -> HashSet<&Selector> {
    constraints.iter().flat_map(find_selectors).collect()
}

pub fn find_gate_query_selector_set<F: Field>(constraints: &[Expression<F>]) -> HashSet<AnyQuery> {
    constraints.iter().flat_map(find_queries).collect()
}

pub fn find_gate_selectors<F: Field>(constraints: &[Expression<F>]) -> Vec<&Selector> {
    let mut selectors: Vec<&Selector> = find_gate_selector_set(constraints)
        .iter()
        .copied()
        .collect();
    selectors.sort_by_key(|lhs| lhs.index());
    selectors
}

pub fn compute_gate_arity<'a, F: Field>(constraints: &'a [Expression<F>]) -> GateArity<'a> {
    let mut queries: Vec<AnyQuery> = find_gate_query_selector_set(constraints)
        .into_iter()
        .collect();
    queries.sort();
    (find_gate_selectors(constraints), queries)
}
