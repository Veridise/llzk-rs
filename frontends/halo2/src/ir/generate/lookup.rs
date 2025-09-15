use crate::{
    expressions::ScopedExpression,
    halo2::Field,
    ir::stmt::IRStmt,
    lookups::callbacks::{LazyLookupTableGenerator, LookupCallbacks},
    synthesis::{CircuitSynthesis, regions::RegionRow},
    utils,
};
use anyhow::Result;

use super::prepend_comment;

pub fn codegen_lookup_invocations<'s, F: Field>(
    syn: &'s CircuitSynthesis<F>,
    region_rows: &[RegionRow<'s, 's, 's, F>],
    lookup_cb: &dyn LookupCallbacks<F>,
) -> Result<Vec<IRStmt<ScopedExpression<'s, 's, F>>>> {
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
