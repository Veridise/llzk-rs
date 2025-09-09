use crate::{
    backend::{
        codegen::{
            inter_region_constraints,
            lookup::{codegen_lookup_invocations, codegen_lookup_modules},
            lower_constraints, scoped_exprs_to_aexpr,
            strats::{load_patterns, lower_gates},
            Codegen, CodegenStrategy,
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
        expr::IRAexpr,
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

/// Code generation strategy that generates the all the code inside the main function.
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
        Row<'s>: ResolversProvider<C::F> + 's,
        RegionRow<'s, 's>: ResolversProvider<C::F> + 's,
    {
        log::debug!(
            "Performing codegen with {} strategy",
            std::any::type_name_of_val(self)
        );

        log::debug!("Generating lookup modules (if desired)");
        codegen_lookup_modules(codegen, syn, lookups)?;

        log::debug!("Generating main body");
        codegen.within_main(syn, move |_| {
            let patterns = load_patterns(gate_cbs);

            let mut stmts: Vec<IRStmt<_>> = vec![];
            let top_level = syn
                .top_level_group()
                .ok_or_else(|| anyhow::anyhow!("Circuit synthesis is missing a top level group"))?;
            let advice_io = top_level.advice_io();
            let instance_io = top_level.instance_io();
            for group in syn.groups() {
                // Do the region stmts first since backends may have more information about names for
                // cells there and some backends do not update the name and always use the first
                // one given.
                stmts.push(
                    chain_lowerable_stmts!(
                        {
                            log::debug!("Lowering gates");
                            lower_gates(
                                syn.gates(),
                                &group.regions(),
                                &patterns,
                                &advice_io,
                                &instance_io,
                            )
                            .and_then(scoped_exprs_to_aexpr)?
                        },
                        {
                            log::debug!("Lowering lookups");
                            codegen_lookup_invocations(syn, group.region_rows().as_slice(), lookups)
                                .and_then(scoped_exprs_to_aexpr)?
                        },
                        {
                            log::debug!("Lowering inter region equality constraints");
                            scoped_exprs_to_aexpr(inter_region_constraints(
                                syn.constraints().edges(),
                                &advice_io,
                                &instance_io,
                            ))
                        }
                    )
                    .collect(),
                );
            }
            Ok(stmts)
        })
    }
}
