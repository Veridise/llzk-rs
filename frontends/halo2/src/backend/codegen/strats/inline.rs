use crate::{
    backend::{
        codegen::{
            inter_region_constraints,
            lookup::{codegen_lookup_invocations, codegen_lookup_modules},
            lower_constraints, Codegen, CodegenStrategy,
        },
        resolvers::ResolversProvider,
    },
    expressions::ScopedExpression,
    gates::{GateRewritePattern, GateScope, RewriteError, RewriteOutput, RewritePatternSet},
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

struct FallbackGateRewriter;

fn create_eq_constraints<'a, F: Field>(
    i: impl IntoIterator<Item = &'a Expression<F>>,
) -> IRStmt<Cow<'a, Expression<F>>> {
    let lhs = i.into_iter().map(Cow::Borrowed);
    let rhs = std::iter::repeat(F::ZERO)
        .map(Expression::Constant)
        .map(Cow::Owned);
    std::iter::zip(lhs, rhs)
        .map(|(lhs, rhs)| IRStmt::constraint(CmpOp::Eq, lhs, rhs))
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
        let rows = gate.rows();
        Ok(rows
            .map(|row| create_eq_constraints(gate.polynomials()).map(&|e| (row, e)))
            .collect())
        //let constraints = std::iter::repeat(gate.polynomials()).map(create_eq_constraints);
        //let rows = gate.rows();
        //Ok(std::iter::zip(constraints, rows)
        //    .map(|(c, r)| c.map(&|e| (r, e)))
        //    .collect())
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
    fn codegen<'c, 's, C>(
        &self,
        codegen: &C,
        syn: &'s CircuitSynthesis<C::F>,
        lookups: &dyn LookupCallbacks<C::F>,
        gate_cbs: &dyn GateCallbacks<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c>,
        Row<'s, C::F>: ResolversProvider<C::F> + 's,
        RegionRow<'s, 's, C::F>: ResolversProvider<C::F> + 's,
    {
        codegen_lookup_modules(codegen, syn, lookups)?;

        codegen.within_main(syn, move |_| {
            let mut patterns = RewritePatternSet::default();
            patterns.extend(gate_cbs.patterns());
            patterns.add(FallbackGateRewriter);
            // Do the region stmts first since backends may have more information about names for
            // cells there and some backends do not update the name and always use the first
            // one given.
            Ok(chain_lowerable_stmts!(
                lower_gates(syn, &patterns)?,
                codegen_lookup_invocations(syn, lookups)?,
                inter_region_constraints(syn)?
            ))
        })
    }
}
