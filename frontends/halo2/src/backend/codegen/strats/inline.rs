
use crate::backend::codegen::lookup::Lookup;
use crate::backend::codegen::{Codegen, CodegenStrategy};
use crate::backend::lowering::Lowering;
use crate::backend::resolvers::{QueryResolver, ResolvedQuery};
use crate::{
    ir::{BinaryBoolOp, CircuitStmt},
    synthesis::{
        regions::RegionRow,
        CircuitSynthesis,
    },
    CircuitWithIO,
};
use anyhow::Result;

#[derive(Default)]
pub struct InlineConstraintsStrat;

impl InlineConstraintsStrat {
    fn define_lookup_modules<'c, C>(&self, backend: &C, syn: &CircuitSynthesis<C::F>) -> Result<()>
    where
        C: Codegen<'c>,
    {
        for lookup in Lookup::load(syn)? {
            let scope = lookup.create_scope(backend, syn)?;
            let resolver = lookup.create_resolver();
            let constraints = lookup.expressions().map(|(input, table)| {
                Ok(CircuitStmt::Constraint(
                    BinaryBoolOp::Eq,
                    scope.lower_expr(input, &resolver, &resolver)?,
                    scope.lower_expr(table, &resolver, &resolver)?,
                ))
            });
            let assumptions = lookup.output_queries().iter().map(|a| {
                resolver
                    .resolve_any_query(a)
                    .map(|rq: ResolvedQuery<C::F>| match rq {
                        ResolvedQuery::Lit(_) => unreachable!(),
                        ResolvedQuery::IO(func_io) => CircuitStmt::AssumeDeterministic(func_io),
                    })
            });

            // TODO: Missing the assume-determinisitic statements.
            backend.lower_stmts(&scope, assumptions.chain(constraints))?;
            backend.on_scope_end(scope)?;
        }
        Ok(())
    }
}

impl CodegenStrategy for InlineConstraintsStrat {
    fn codegen<'c, C>(&self, codegen: &C, syn: &CircuitSynthesis<C::F>) -> Result<()>
    where
        C: Codegen<'c>,
    {
        self.define_lookup_modules(codegen, syn)?;

        codegen.within_main(syn, move |scope| {
            let region_rows = || {
                syn.regions().into_iter().flat_map(|r| {
                    r.rows()
                        .map(move |row| RegionRow::new(r, row, syn.advice_io(), syn.instance_io()))
                })
            };
            let lookups = Lookup::load(syn)?
                .into_iter()
                .flat_map(|l| region_rows().map(move |r| l.create_call_stmt(scope, &r)));
            // Do the region stmts first since backends may have more information about names for
            // cells there and some backends do not update the name and always use the first
            // one given.
            syn.region_gates()
                .flat_map(|(gate, r)| {
                    scope.lower_constraints(gate, r, r.header(), Some(r.row_number()))
                })
                .chain(lookups)
                .chain(self.inter_region_constraints(scope, syn))
                .collect::<Result<Vec<_>>>()
        })
    }
}
