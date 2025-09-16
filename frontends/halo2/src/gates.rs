use std::{borrow::Cow, cmp::Ordering, collections::HashSet, hash::Hash, ops::Range};

use crate::{
    expressions::{constant_folding::ConstantFolding, rewriter::rewrite_expr, ScopedExpression},
    halo2::*,
    ir::stmt::IRStmt,
    resolvers::FixedQueryResolver,
    synthesis::regions::{RegionData, RegionRow},
};

/// Error emitted by the patterns that can indicate either that the pattern didn't match or that it
/// failed.
#[derive(Debug)]
pub enum RewriteError {
    /// Indicates that the pattern didn't match the gate.
    NoMatch,
    /// Indicates that the pattern failed.
    Err(anyhow::Error),
}

/// Result of constant-folding an expression for `n` rows.
pub type FoldedExpressions<F> = Vec<(usize, Expression<F>)>;

/// Scope in which a gate is being called
#[derive(Copy, Clone)]
pub struct GateScope<'syn, 'io, F>
where
    F: Field,
{
    gate: &'syn Gate<F>,
    region: RegionData<'syn>,
    /// The bounds are [start,end).
    row_bounds: (usize, usize),
    advice_io: &'io crate::io::AdviceIO,
    instance_io: &'io crate::io::InstanceIO,
    fqr: &'syn dyn FixedQueryResolver<F>,
}

impl<'syn, 'io, F: Field> GateScope<'syn, 'io, F> {
    /// Constructs a new gate scope.
    ///
    /// Since this class is passed to a callback its constructor is protected.
    pub(crate) fn new(
        gate: &'syn Gate<F>,
        region: RegionData<'syn>,
        row_bounds: (usize, usize),
        advice_io: &'io crate::io::AdviceIO,
        instance_io: &'io crate::io::InstanceIO,
        fqr: &'syn dyn FixedQueryResolver<F>,
    ) -> Self {
        Self {
            gate,
            region,
            row_bounds,
            advice_io,
            instance_io,
            fqr,
        }
    }

    pub(crate) fn region(&self) -> RegionData<'syn> {
        self.region
    }

    pub(crate) fn region_row(&self, row: usize) -> anyhow::Result<RegionRow<'syn, 'io, 'syn, F>> {
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
            self.fqr,
        ))
    }

    pub(crate) fn region_rows(&self) -> impl Iterator<Item = RegionRow<'syn, 'io, 'syn, F>> {
        self.rows().map(|row| {
            RegionRow::new(
                self.region(),
                row,
                self.advice_io,
                self.instance_io,
                self.fqr,
            )
        })
    }

    /// Returns the name assigned to the gate.
    pub fn gate_name(&self) -> &str {
        self.gate.name()
    }

    /// Returns the polynomials defined during circuit configuration.
    pub fn polynomials(&self) -> &'syn [Expression<F>] {
        self.gate.polynomials()
    }

    /// Returns the list of polynomials once per row. The polynomials per row are constant-folded
    /// first.
    pub fn polynomials_per_row(
        &self,
    ) -> anyhow::Result<Vec<(&'syn Expression<F>, FoldedExpressions<F>)>> {
        self.polynomials()
            .iter()
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
        Ok(rewrite_expr(
            scoped.as_ref(),
            &[&ConstantFolding::new(scoped.resolvers())],
        ))
    }

    /// Returns the name of the region where this gate was called.
    pub fn region_name(&self) -> &str {
        self.region.name()
    }

    /// Returns the index of the region where this gate was called.
    pub fn region_index(&self) -> Option<RegionIndex> {
        self.region.index()
    }

    /// Returns a string summary of the region.
    ///
    /// It's intended for debugging purposes and the
    /// text representation should not be relied upon.
    pub fn region_header(&self) -> impl ToString {
        self.region.header()
    }

    /// Returns the first row of the region.
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

    /// Returns the rows in the region.
    pub fn rows(&self) -> Range<usize> {
        (self.row_bounds.0)..(self.row_bounds.1)
    }
}

impl<F: PrimeField> std::fmt::Debug for GateScope<'_, '_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GateScope")
            .field("gate", &self.gate)
            .field("region", &self.region)
            .field("row_bounds", &self.row_bounds)
            .field("advice_io", &self.advice_io)
            .field("instance_io", &self.instance_io)
            .finish()
    }
}

/// The type used for rewriting the gates. Each expression has an associated row that is used as
/// the base offset on the queries.
pub type RewriteOutput<'syn, F> = IRStmt<(usize, Cow<'syn, Expression<F>>)>;

/// Implementations of this trait can selectively rewrite a gate when lowering the circuit.
///
/// The rewrites performed by these patterns should be semantics preserving.
pub trait GateRewritePattern<F> {
    /// Checks if the gate matches the pattern.
    ///
    /// Returns Ok(()) if the pattern matched.
    #[allow(unused_variables)]
    fn match_gate(&self, gate: GateScope<F>) -> Result<(), RewriteError>
    where
        F: Field,
    {
        panic!("Implement match_gate and rewrite_gate OR match_and_rewrite")
    }

    /// Performs the rewriting of the gate.
    #[allow(unused_variables)]
    fn rewrite_gate<'syn>(
        &self,
        gate: GateScope<'syn, '_, F>,
    ) -> Result<RewriteOutput<'syn, F>, anyhow::Error>
    where
        F: Field,
    {
        panic!("Implement match_gate and rewrite_gate OR match_and_rewrite")
    }

    /// Checks if the gate matches the pattern and then performs the rewriting.
    fn match_and_rewrite<'syn>(
        &self,
        gate: GateScope<'syn, '_, F>,
    ) -> Result<RewriteOutput<'syn, F>, RewriteError>
    where
        F: Field,
    {
        self.match_gate(gate)?;
        self.rewrite_gate(gate).map_err(RewriteError::Err)
    }
}

/// User configuration for the lowering process of gates.
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

/// Default gate callbacks.
pub(crate) struct DefaultGateCallbacks;

impl<F> GateCallbacks<F> for DefaultGateCallbacks {
    fn patterns(&self) -> Vec<Box<dyn GateRewritePattern<F>>>
    where
        F: Field,
    {
        vec![]
    }
}

/// A set of rewrite patterns.
#[derive(Default)]
pub(crate) struct RewritePatternSet<F>(Vec<Box<dyn GateRewritePattern<F>>>);

impl<F> RewritePatternSet<F> {
    /// Adds a pattern to the set.
    pub fn add(&mut self, p: impl GateRewritePattern<F> + 'static) {
        self.0.push(Box::new(p))
    }
}

impl<F> Extend<Box<dyn GateRewritePattern<F>>> for RewritePatternSet<F> {
    fn extend<T: IntoIterator<Item = Box<dyn GateRewritePattern<F>>>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

impl<F> GateRewritePattern<F> for RewritePatternSet<F> {
    fn match_and_rewrite<'syn>(
        &self,
        gate: GateScope<'syn, '_, F>,
    ) -> Result<RewriteOutput<'syn, F>, RewriteError>
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
