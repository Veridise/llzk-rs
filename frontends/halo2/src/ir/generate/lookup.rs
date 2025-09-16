use crate::{
    expressions::ScopedExpression,
    halo2::Field,
    ir::stmt::IRStmt,
    lookups::callbacks::{LazyLookupTableGenerator, LookupCallbacks},
    synthesis::{regions::RegionRow, CircuitSynthesis},
    utils,
};
use anyhow::Result;

use super::prepend_comment;

pub fn codegen_lookup_invocations<'sco, 'syn, 'ctx, 'cb, F>(
    syn: &'syn CircuitSynthesis<F>,
    region_rows: &[RegionRow<'syn, 'ctx, 'syn, F>],
    lookup_cb: &'cb dyn LookupCallbacks<F>,
) -> Result<Vec<IRStmt<ScopedExpression<'syn, 'sco, F>>>>
where
    'syn: 'sco,
    'ctx: 'sco + 'syn,
    'cb: 'sco + 'syn,
    F: Field,
{
    utils::product(syn.lookups(), region_rows)
        .map(|(lookup, rr)| {
            let table = LazyLookupTableGenerator::new(|| {
                syn.tables_for_lookup(&lookup)
                    .map(|table| table.into_boxed_slice())
            });
            lookup_cb.on_lookup(lookup, &table).map(|stmts| {
                let comment = IRStmt::comment(format!("{lookup} @ {}", rr.header()));
                let stmts = stmts
                    .into_iter()
                    .map(|stmt| stmt.map(&|e| ScopedExpression::from_cow(e, *rr)));
                prepend_comment(stmts.collect(), || comment)
            })
        })
        .collect()
}
