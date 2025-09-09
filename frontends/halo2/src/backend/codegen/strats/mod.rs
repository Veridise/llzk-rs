use crate::backend::resolvers::FixedQueryResolver;
use crate::expressions::utils::ExprDebug;
use crate::expressions::ScopedExpression;
use crate::gates::{find_selectors, RewritePatternSet};
use crate::halo2::Field;
use crate::halo2::{Advice, Expression, Gate, Instance};
use crate::ir::stmt::IRStmt;
use crate::ir::CmpOp;
use crate::synthesis::regions::RegionData;
use crate::{
    utils, CircuitIO, GateCallbacks, GateRewritePattern, GateScope, RegionRowLike as _,
    RewriteError, RewriteOutput,
};
use anyhow::Result;
use std::{borrow::Cow, result::Result as StdResult};

use super::prepend_comment;

pub mod groups;
pub mod inline;

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
                    .iter()
                    .filter(move |e| {
                        let set = find_selectors(e);
                        if self.ignore_disabled_gates && row.gate_is_disabled(&set) {
                            log::debug!(
                                "Expression {:?} was ignored because its selectors are disabled",
                                ExprDebug(e)
                            );
                            return false;
                        }
                        true
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
    fqr: &'a dyn FixedQueryResolver<F>,
) -> Result<Vec<IRStmt<ScopedExpression<'a, 'a, F>>>> {
    utils::product(regions, gates)
        .map(|(r, g)| {
            let rows = r.rows();
            let scope = GateScope::new(g, *r, (rows.start, rows.end), advice_io, instance_io, fqr);

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
