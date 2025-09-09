use crate::{
    backend::{
        codegen::{
            inter_region_constraints,
            lookup::codegen_lookup_invocations,
            scoped_exprs_to_aexpr,
            strats::{load_patterns, lower_gates},
            Codegen, CodegenStrategy,
        },
        resolvers::ResolversProvider,
    },
    ir::stmt::{chain_lowerable_stmts, IRStmt},
    lookups::callbacks::LookupCallbacks,
    synthesis::{
        regions::{RegionRow, Row},
        CircuitSynthesis,
    },
    GateCallbacks,
};
use anyhow::Result;

/// Code generation strategy that generates the all the code inside the main function.
#[derive(Default)]
#[allow(dead_code)]
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
        Row<'s, 's, C::F>: ResolversProvider<C::F> + 's,
        RegionRow<'s, 's, 's, C::F>: ResolversProvider<C::F> + 's,
    {
        log::debug!(
            "Performing codegen with {} strategy",
            std::any::type_name_of_val(self)
        );

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
                                advice_io,
                                instance_io,
                                syn.fixed_query_resolver(),
                            )
                            .and_then(scoped_exprs_to_aexpr)?
                        },
                        {
                            log::debug!("Lowering lookups");
                            codegen_lookup_invocations(
                                syn,
                                group.region_rows(syn.fixed_query_resolver()).as_slice(),
                                lookups,
                            )
                            .and_then(scoped_exprs_to_aexpr)?
                        },
                        {
                            log::debug!("Lowering inter region equality constraints");
                            scoped_exprs_to_aexpr(inter_region_constraints(
                                syn.constraints().edges(),
                                advice_io,
                                instance_io,
                                syn.fixed_query_resolver(),
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
