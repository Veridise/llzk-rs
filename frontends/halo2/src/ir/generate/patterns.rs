use crate::expressions::utils::ExprDebug;
use crate::gates::{find_selectors, RewritePatternSet};
use crate::halo2::Expression;
use crate::halo2::Field;
use crate::ir::stmt::IRStmt;
use crate::ir::CmpOp;
use crate::{GateCallbacks, GateRewritePattern, GateScope, RewriteError, RewriteOutput};
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
pub fn load_patterns<F: Field>(gate_cbs: &dyn GateCallbacks<F>) -> RewritePatternSet<F> {
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
