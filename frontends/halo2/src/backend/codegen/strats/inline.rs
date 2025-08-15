use crate::{
    backend::{
        codegen::{
            inter_region_constraints,
            lookup::{codegen_lookup_invocations, codegen_lookup_modules},
            lower_constraints, Codegen, CodegenStrategy,
        },
        resolvers::ResolversProvider,
    },
    expressions::{utils::ExprDebug, ScopedExpression},
    gates::{
        find_selectors, GateRewritePattern, GateScope, RewriteError, RewriteOutput,
        RewritePatternSet,
    },
    halo2::{Expression, Field},
    ir::{
        stmt::{chain_lowerable_stmts, IRStmt},
        CmpOp,
    },
    lookups::callbacks::LookupCallbacks,
    synthesis::{
        regions::{RegionRow, RegionRowLike as _, Row},
        CircuitSynthesis,
    },
    GateCallbacks,
};
use anyhow::Result;
use std::{borrow::Cow, result::Result as StdResult};

fn header_comments<F: Field, S: ToString>(s: S) -> Vec<IRStmt<(F,)>> {
    s.to_string().lines().map(IRStmt::comment).collect()
}

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

fn zero<'a, F: Field>() -> Cow<'a, Expression<F>> {
    Cow::Owned(Expression::Constant(F::ZERO))
}

fn create_eq_constraints<'a, F: Field>(
    i: impl IntoIterator<Item = &'a Expression<F>>,
    row: usize,
) -> RewriteOutput<'a, F> {
    i.into_iter()
        .map(Cow::Borrowed)
        .map(|lhs| IRStmt::constraint(CmpOp::Eq, lhs, zero()))
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
            .map(move |row| {
                log::debug!("Creating constraints for row {}", row.row_number());

                let filtered_polys = gate.polynomials().into_iter().filter_map(|e| {
                    let set = find_selectors(e);
                    if self.ignore_disabled_gates && row.gate_is_disabled(&set) {
                        log::debug!(
                            "Expression {:?} was ignored because its selectors are disabled",
                            ExprDebug(e)
                        );
                        return None;
                    }
                    Some(e)
                });
                create_eq_constraints(filtered_polys, row.row_number())
            })
            .collect())
    }
}

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

fn lower_gates<'s, F: Field>(
    syn: &'s CircuitSynthesis<F>,
    patterns: &RewritePatternSet<F>,
) -> Result<Vec<IRStmt<ScopedExpression<'s, 's, F>>>> {
    syn.gate_scopes()
        .map(|scope: GateScope<'s, F>| {
            patterns
                .match_and_rewrite(scope)
                .map_err(|e| make_error(e, scope))
                .and_then(|stmt| {
                    stmt.try_map(&|(row, expr)| {
                        let rr = scope.region_row(row)?;
                        Ok(ScopedExpression::from_cow(expr, rr))
                    })
                })
                .map(|stmt| {
                    if stmt.is_empty() {
                        return stmt;
                    }
                    [
                        IRStmt::comment(format!(
                            "gate '{}' @ {} @ rows {}..={}",
                            scope.gate_name(),
                            scope.region_header().to_string(),
                            scope.start_row(),
                            scope.end_row()
                        )),
                        stmt,
                    ]
                    .into_iter()
                    .collect()
                })
        })
        .collect::<Result<Vec<_>>>()
}

#[derive(Default)]
pub struct InlineConstraintsStrat {}

impl CodegenStrategy for InlineConstraintsStrat {
    fn codegen<'c: 'st, 's, 'st, C>(
        &self,
        codegen: &C,
        syn: &'s CircuitSynthesis<C::F>,
        lookups: &dyn LookupCallbacks<C::F>,
        gate_cbs: &dyn GateCallbacks<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c, 'st>,
        Row<'s, C::F>: ResolversProvider<C::F> + 's,
        RegionRow<'s, 's, C::F>: ResolversProvider<C::F> + 's,
    {
        log::debug!(
            "Performing codegen with {} strategy",
            std::any::type_name_of_val(self)
        );

        log::debug!("Generating lookup modules (if desired)");
        codegen_lookup_modules(codegen, syn, lookups)?;

        log::debug!("Generating main body");
        codegen.within_main(syn, move |_| {
            let mut patterns = RewritePatternSet::default();
            let user_patterns = gate_cbs.patterns();
            log::debug!("Loading {} user patterns", user_patterns.len());
            patterns.extend(user_patterns);
            log::debug!(
                "Loading fallback pattern {}",
                std::any::type_name::<FallbackGateRewriter>()
            );
            patterns.add(FallbackGateRewriter::new(gate_cbs.ignore_disabled_gates()));
            // Do the region stmts first since backends may have more information about names for
            // cells there and some backends do not update the name and always use the first
            // one given.
            Ok(chain_lowerable_stmts!(
                {
                    log::debug!("Lowering gates");
                    lower_gates(syn, &patterns)?
                },
                {
                    log::debug!("Lowering lookups");
                    codegen_lookup_invocations(syn, lookups)?
                },
                {
                    log::debug!("Lowering inter region equality constraints");
                    inter_region_constraints(syn)?
                }
            ))
        })
    }
}
