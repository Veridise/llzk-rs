use crate::backend::func::{ArgNo, FieldId, FuncIO};
use crate::backend::resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver};
use crate::expressions::utils::ExprDebug;
use crate::expressions::ScopedExpression;
use crate::gates::{find_selectors, RewritePatternSet};
use crate::halo2::{Advice, Expression, Gate, Instance};
use crate::ir::stmt::IRStmt;
use crate::ir::CmpOp;
use crate::synthesis::regions::RegionData;
use crate::{
    gates::AnyQuery,
    halo2::{AdviceQuery, Field, FixedQuery, InstanceQuery, Selector},
    synthesis::regions::FQN,
};
use crate::{
    utils, CircuitIO, GateCallbacks, GateRewritePattern, GateScope, RegionRowLike as _,
    RewriteError, RewriteOutput,
};
use anyhow::{anyhow, Result};
use std::{borrow::Cow, result::Result as StdResult};

use super::prepend_comment;

//pub mod call_gates;
pub mod groups;
pub mod inline;

#[derive(Copy, Clone)]
enum IO {
    I(usize),
    O(usize),
}

#[derive(Clone)]
pub struct GateScopedResolver<'a> {
    pub selectors: Vec<&'a Selector>,
    pub queries: Vec<AnyQuery>,
    pub outputs: Vec<AnyQuery>,
}

fn resolve<'a, A, B, I, O>(mut it: I, b: &B, err: &'static str) -> Result<O>
where
    A: PartialEq<B> + 'a,
    I: Iterator<Item = (&'a A, IO)>,
    O: From<FuncIO>,
{
    it.find_map(|(a, io)| -> Option<FuncIO> {
        if a == b {
            Some(match io {
                IO::I(idx) => ArgNo::from(idx).into(),
                IO::O(idx) => FieldId::from(idx).into(),
            })
        } else {
            None
        }
    })
    .map(From::from)
    .ok_or(anyhow!(err))
}

impl<'a> GateScopedResolver<'a> {
    fn selectors(&self) -> impl Iterator<Item = (&'a Selector, IO)> {
        self.selectors
            .iter()
            .copied()
            .enumerate()
            .map(|(idx, s)| (s, IO::I(idx)))
    }

    fn io_queries<'q>(&'q self) -> impl Iterator<Item = (&'q AnyQuery, IO)> {
        let input_base = self.selectors.len();
        self.queries
            .iter()
            .enumerate()
            .map(move |(idx, q)| (q, IO::I(idx + input_base)))
            .chain(
                self.outputs
                    .iter()
                    .enumerate()
                    .map(|(idx, q)| (q, IO::O(idx))),
            )
    }
}

impl<F: Field> QueryResolver<F> for GateScopedResolver<'_> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        resolve(self.io_queries(), query, "Query as argument not found")
    }

    fn resolve_advice_query(
        &self,
        query: &AdviceQuery,
    ) -> Result<(ResolvedQuery<F>, Option<Cow<'_, FQN>>)> {
        Ok((
            resolve(self.io_queries(), query, "Query as argument not found")?,
            None,
        ))
    }

    fn resolve_instance_query(&self, query: &InstanceQuery) -> Result<ResolvedQuery<F>> {
        resolve(self.io_queries(), query, "Query as argument not found")
    }
}

impl SelectorResolver for GateScopedResolver<'_> {
    fn resolve_selector(&self, selector: &Selector) -> Result<ResolvedSelector> {
        resolve(self.selectors(), selector, "Selector as argument not found").and_then(
            |io: FuncIO| match io {
                FuncIO::Arg(arg) => Ok(ResolvedSelector::Arg(arg)),
                _ => anyhow::bail!("Cannot get a selector as anything other than an argument"),
            },
        )
    }
}

/// Default gate pattern that transforms each polynomial in a gate into an equality statement for
/// each row in the region.
struct FallbackGateRewriter {
    ignore_disabled_gates: bool,
}

impl FallbackGateRewriter {
    pub fn new(ignore_disabled_gates: bool) -> Self {
        Self {
            ignore_disabled_gates,
        }
    }
}

fn create_eq_constraints<'a, F: Field>(
    i: impl IntoIterator<Item = &'a Expression<F>>,
    row: usize,
) -> RewriteOutput<'a, F> {
    i.into_iter()
        .map(Cow::Borrowed)
        .map(|lhs| IRStmt::constraint(CmpOp::Eq, lhs, Cow::Owned(Expression::Constant(F::ZERO))))
        .map(|s| s.map(&|e: Cow<'a, _>| (row, e)))
        .collect()
}

impl<F> GateRewritePattern<F> for FallbackGateRewriter {
    fn match_gate<'a>(&self, _gate: GateScope<'a, F>) -> StdResult<(), RewriteError>
    where
        F: Field,
    {
        Ok(()) // Match all
    }

    fn rewrite_gate<'a>(
        &self,
        gate: GateScope<'a, F>,
    ) -> StdResult<RewriteOutput<'a, F>, anyhow::Error>
    where
        F: Field,
    {
        log::debug!(
            "Generating gate '{}' on region '{}' with the fallback rewriter",
            gate.gate_name(),
            gate.region_name()
        );
        let rows = gate.region_rows();
        log::debug!("The region has {} rows", gate.rows().count());
        Ok(rows
            .flat_map(move |row| {
                log::debug!("Creating constraints for row {}", row.row_number());

                gate.polynomials()
                    .into_iter()
                    .filter_map(move |e| {
                        let set = find_selectors(e);
                        if self.ignore_disabled_gates && row.gate_is_disabled(&set) {
                            log::debug!(
                                "Expression {:?} was ignored because its selectors are disabled",
                                ExprDebug(e)
                            );
                            return None;
                        }
                        Some(e)
                    })
                    .map(Cow::Borrowed)
                    .map(|lhs| {
                        IRStmt::constraint(
                            CmpOp::Eq,
                            lhs,
                            Cow::Owned(Expression::Constant(F::ZERO)),
                        )
                    })
                    .map(move |s| s.map(&|e: Cow<'a, _>| (row.row_number(), e)))
                //.collect()
            })
            .collect())
    }
}

/// Configures a rewrite pattern set from patterns potentially provided by the user and
/// the fallback pattern for gates that don't require special handling.
fn load_patterns<F: Field>(gate_cbs: &dyn GateCallbacks<F>) -> RewritePatternSet<F> {
    let mut patterns = RewritePatternSet::default();
    let user_patterns = gate_cbs.patterns();
    log::debug!("Loading {} user patterns", user_patterns.len());
    patterns.extend(user_patterns);
    log::debug!(
        "Loading fallback pattern {}",
        std::any::type_name::<FallbackGateRewriter>()
    );
    patterns.add(FallbackGateRewriter::new(gate_cbs.ignore_disabled_gates()));
    patterns
}

/// If the rewrite error is [`RewriteError::NoMatch`] returns an error
/// that the gate in scope did not match any pattern. If it is [`RewriteError::Err`]
/// forwards the inner error.
fn make_error<F>(e: RewriteError, scope: GateScope<F>) -> anyhow::Error
where
    F: Field,
{
    match e {
        RewriteError::NoMatch => anyhow::anyhow!(
            "Gate '{}' on region {} '{}' did not match any pattern",
            scope.gate_name(),
            scope
                .region_index()
                .as_deref()
                .map(ToString::to_string)
                .unwrap_or("unk".to_string()),
            scope.region_name()
        ),
        RewriteError::Err(error) => anyhow::anyhow!(error),
    }
}

/// Uses the given rewrite patterns to lower the gates on each region.
fn lower_gates<'a, F: Field>(
    gates: &'a [Gate<F>],
    regions: &'a [RegionData<'a>],
    patterns: &RewritePatternSet<F>,
    advice_io: &'a CircuitIO<Advice>,
    instance_io: &'a CircuitIO<Instance>,
) -> Result<Vec<IRStmt<ScopedExpression<'a, 'a, F>>>> {
    utils::product(regions, gates)
        .map(|(r, g)| {
            let rows = r.rows();
            let scope = GateScope::new(g, *r, (rows.start, rows.end), advice_io, instance_io);

            let header = IRStmt::comment(format!(
                "gate '{}' @ {} @ rows {}..={}",
                scope.gate_name(),
                scope.region_header().to_string(),
                scope.start_row(),
                scope.end_row()
            ));
            patterns
                .match_and_rewrite(scope)
                .map_err(|e| make_error(e, scope))
                .and_then(|stmt| {
                    stmt.try_map(&|(row, expr)| {
                        let rr = scope.region_row(row)?;
                        Ok(ScopedExpression::from_cow(expr, rr))
                    })
                })
                .map(|stmt| prepend_comment(stmt, || header))
        })
        .collect()
}
