//! Types and traits related to gates.

use std::{borrow::Cow, cell::RefCell, ops::Range};

use crate::{
    expressions::{
        EvalExpression, EvaluableExpr, ExprBuilder, ExpressionInfo, ScopedExpression,
        constant_folding::ConstantFolding,
    },
    halo2::*,
    info_traits::GateInfo,
    io::{AdviceIO, InstanceIO},
    ir::stmt::IRStmt,
    resolvers::FixedQueryResolver,
    synthesis::regions::{RegionData, RegionRow},
};

/// Information about a gate in the constraint system.
///
/// Is parameterized by the expression type used to represent polynomials.
pub(crate) struct Gate<E> {
    name: String,
    polynomials: Vec<E>,
}

impl<E> Gate<E> {
    /// Creates a new gate.
    pub fn new<F: Field>(info: &dyn GateInfo<E>) -> Self
    where
        E: Clone,
    {
        Self {
            name: info.name().to_string(),
            polynomials: info.polynomials().into_iter().cloned().collect(),
        }
    }

    /// Returns the name of the gate.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the polynomials of the gate.
    pub fn polynomials(&self) -> &[E] {
        &self.polynomials
    }
}

impl<E: std::fmt::Debug> std::fmt::Debug for Gate<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gate")
            .field("name", &self.name)
            .field("polynomials", &self.polynomials)
            .finish()
    }
}

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
pub type FoldedExpressions<E> = Vec<(usize, E)>;

/// Scope in which a gate is being called
pub struct GateScope<'syn, 'io, F, E>
where
    F: Field,
{
    gate: &'syn Gate<E>,
    region: RegionData<'syn>,
    /// The bounds are [start,end).
    row_bounds: (usize, usize),
    advice_io: &'io AdviceIO,
    instance_io: &'io InstanceIO,
    fqr: &'syn dyn FixedQueryResolver<F>,
}

impl<'syn, 'io, F: Field, E> GateScope<'syn, 'io, F, E> {
    /// Constructs a new gate scope.
    ///
    /// Since this class is passed to a callback its constructor is protected.
    pub(crate) fn new(
        gate: &'syn Gate<E>,
        region: RegionData<'syn>,
        row_bounds: (usize, usize),
        advice_io: &'io AdviceIO,
        instance_io: &'io InstanceIO,
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
    pub fn polynomials(&self) -> &'syn [E] {
        self.gate.polynomials()
    }

    /// Returns the list of polynomials once per row. The polynomials per row are constant-folded
    /// first.
    pub fn polynomials_per_row(&self) -> anyhow::Result<Vec<(&'syn E, FoldedExpressions<E>)>>
    where
        E: Clone + EvaluableExpr<F> + ExpressionInfo + ExprBuilder<F>,
    {
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

    fn fold_polynomial_in_row(&self, e: &E, row: usize) -> anyhow::Result<E>
    where
        E: Clone + EvaluableExpr<F> + ExpressionInfo + ExprBuilder<F>,
    {
        let region_row = self.region_row(row)?;
        let scoped = ScopedExpression::from_ref(e, region_row);
        Ok(ConstantFolding::new(scoped.resolvers()).constant_fold(scoped.as_ref()))
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

impl<F: Field, E> Copy for GateScope<'_, '_, F, E> {}

impl<F: Field, E> Clone for GateScope<'_, '_, F, E> {
    fn clone(&self) -> Self {
        Self {
            gate: self.gate,
            region: self.region,
            row_bounds: self.row_bounds,
            advice_io: self.advice_io,
            instance_io: self.instance_io,
            fqr: self.fqr,
        }
    }
}

impl<F: PrimeField, E: std::fmt::Debug> std::fmt::Debug for GateScope<'_, '_, F, E> {
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
pub type RewriteOutput<'syn, E> = IRStmt<(usize, Cow<'syn, E>)>;

/// Implementations of this trait can selectively rewrite a gate when lowering the circuit.
///
/// The rewrites performed by these patterns should be semantics preserving.
pub trait GateRewritePattern<F, E> {
    /// Checks if the gate matches the pattern.
    ///
    /// Returns Ok(()) if the pattern matched.
    #[allow(unused_variables)]
    fn match_gate(&self, gate: GateScope<F, E>) -> Result<(), RewriteError>
    where
        F: Field,
    {
        panic!("Implement match_gate and rewrite_gate OR match_and_rewrite")
    }

    /// Performs the rewriting of the gate.
    #[allow(unused_variables)]
    fn rewrite_gate<'syn>(
        &self,
        gate: GateScope<'syn, '_, F, E>,
    ) -> Result<RewriteOutput<'syn, E>, anyhow::Error>
    where
        F: Field,
        E: Clone,
    {
        panic!("Implement match_gate and rewrite_gate OR match_and_rewrite")
    }

    /// Checks if the gate matches the pattern and then performs the rewriting.
    fn match_and_rewrite<'syn>(
        &self,
        gate: GateScope<'syn, '_, F, E>,
    ) -> Result<RewriteOutput<'syn, E>, RewriteError>
    where
        F: Field,
        E: Clone,
    {
        self.match_gate(gate)?;
        self.rewrite_gate(gate).map_err(RewriteError::Err)
    }
}

/// User configuration for the lowering process of gates.
pub trait GateCallbacks<F, E> {
    /// Asks wether a gate's polynomial whose selectors are all disabled for a given region should be emitted or
    /// not. Defaults to true.
    fn ignore_disabled_gates(&self) -> bool {
        true
    }

    /// Asks for a list of patterns that are checked before the default ones.
    fn patterns(&self) -> Vec<Box<dyn GateRewritePattern<F, E>>>
    where
        F: Field;
}

/// Default gate callbacks.
pub(crate) struct DefaultGateCallbacks;

impl<F, E> GateCallbacks<F, E> for DefaultGateCallbacks {
    fn patterns(&self) -> Vec<Box<dyn GateRewritePattern<F, E>>>
    where
        F: Field,
    {
        vec![]
    }
}

/// A set of rewrite patterns.
pub(crate) struct RewritePatternSet<F, E>(Vec<Box<dyn GateRewritePattern<F, E>>>);

impl<F, E> RewritePatternSet<F, E> {
    /// Adds a pattern to the set.
    pub fn add(&mut self, p: impl GateRewritePattern<F, E> + 'static) {
        self.0.push(Box::new(p))
    }
}

impl<F, E> Default for RewritePatternSet<F, E> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<F, E> Extend<Box<dyn GateRewritePattern<F, E>>> for RewritePatternSet<F, E> {
    fn extend<T: IntoIterator<Item = Box<dyn GateRewritePattern<F, E>>>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

impl<F, E> GateRewritePattern<F, E> for RewritePatternSet<F, E> {
    fn match_and_rewrite<'syn>(
        &self,
        gate: GateScope<'syn, '_, F, E>,
    ) -> Result<RewriteOutput<'syn, E>, RewriteError>
    where
        F: Field,
        E: Clone,
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
            RewriteError::Err(anyhow::anyhow!(
                errors
                    .into_iter()
                    .flat_map(|e: anyhow::Error| [e.to_string(), "\n".to_string()])
                    .collect::<String>()
            ))
        })
    }
}

pub(crate) type SelectorSet = bit_set::BitSet;

pub(crate) fn find_selectors<F: Field, E: EvaluableExpr<F>>(poly: &E) -> SelectorSet {
    struct Eval(RefCell<SelectorSet>);

    impl<F> EvalExpression<F> for Eval {
        type Output = ();

        fn selector(&self, selector: &crate::halo2::Selector) -> Self::Output {
            self.0.borrow_mut().insert(selector.index());
        }

        fn constant(&self, _: &F) -> Self::Output {}
        fn fixed(&self, _: &crate::halo2::FixedQuery) -> Self::Output {}
        fn advice(&self, _: &crate::halo2::AdviceQuery) -> Self::Output {}
        fn instance(&self, _: &crate::halo2::InstanceQuery) -> Self::Output {}
        fn challenge(&self, _: &crate::halo2::Challenge) -> Self::Output {}
        fn negated(&self, _: Self::Output) -> Self::Output {}
        fn sum(&self, _: Self::Output, _: Self::Output) -> Self::Output {}
        fn product(&self, _: Self::Output, _: Self::Output) -> Self::Output {}
        fn scaled(&self, _: Self::Output, _: &F) -> Self::Output {}
    }
    let e = Eval(Default::default());
    poly.evaluate(&e);
    e.0.take()
}
