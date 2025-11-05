use ff::Field;

use crate::{
    expressions::{EvaluableExpr, ExprBuilder},
    gates::{
        GateCallbacks, GateRewritePattern, GateScope, RewriteError, RewriteOutput,
        RewritePatternSet, find_selectors,
    },
    ir::{CmpOp, stmt::IRStmt},
};
use std::{borrow::Cow, result::Result as StdResult};

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

impl<F, E> GateRewritePattern<F, E> for FallbackGateRewriter
where
    E: std::fmt::Debug + EvaluableExpr<F> + ExprBuilder<F>,
{
    fn match_gate<'syn>(&self, _gate: GateScope<'syn, '_, F, E>) -> StdResult<(), RewriteError>
    where
        F: Field,
    {
        Ok(()) // Match all
    }

    fn rewrite_gate<'syn>(
        &self,
        gate: GateScope<'syn, '_, F, E>,
    ) -> StdResult<RewriteOutput<'syn, E>, anyhow::Error>
    where
        F: Field,
        E: Clone,
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
                        let set = find_selectors(*e);
                        if self.ignore_disabled_gates && row.gate_is_disabled(&set) {
                            log::debug!(
                                "Expression {e:?} was ignored because its selectors are disabled",
                            );
                            return false;
                        }
                        true
                    })
                    .map(Cow::Borrowed)
                    .map(|lhs| IRStmt::constraint(CmpOp::Eq, lhs, Cow::Owned(E::constant(F::ZERO))))
                    .map(move |s| s.map(&|e: Cow<'syn, _>| (row.row_number(), e)))
                //.collect()
            })
            .collect())
    }
}

/// Configures a rewrite pattern set from patterns potentially provided by the user and
/// the fallback pattern for gates that don't require special handling.
pub fn load_patterns<F, E>(gate_cbs: &dyn GateCallbacks<F, E>) -> RewritePatternSet<F, E>
where
    F: Field,
    E: ExprBuilder<F> + EvaluableExpr<F> + std::fmt::Debug,
{
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
