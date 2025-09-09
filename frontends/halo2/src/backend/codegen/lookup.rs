use std::{convert::identity, ops::BitOr};

use crate::{
    expressions::ScopedExpression,
    halo2::{Expression, Field, FixedQuery},
    ir::stmt::IRStmt,
    lookups::callbacks::{LazyLookupTableGenerator, LookupCallbacks},
    synthesis::{regions::RegionRow, CircuitSynthesis},
    utils,
};
use anyhow::{anyhow, Result};

use super::prepend_comment;

pub fn codegen_lookup_invocations<'s, F: Field>(
    syn: &'s CircuitSynthesis<F>,
    region_rows: &'s [RegionRow<'s, 's, 's, F>],
    lookup_cb: &'s dyn LookupCallbacks<F>,
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

pub fn query_from_table_expr<F: Field>(e: &Expression<F>) -> Result<FixedQuery> {
    match e {
        Expression::Fixed(fixed_query) => Ok(*fixed_query),
        _ => Err(anyhow!(
            "Table row expressions can only be fixed cell queries"
        )),
    }
}

pub fn contains_fixed<F: Field>(e: &&Expression<F>) -> bool {
    fn false_cb<I>(_: I) -> bool {
        false
    }
    e.evaluate(
        &false_cb,
        &false_cb,
        &|_| true,
        &false_cb,
        &false_cb,
        &false_cb,
        &identity,
        &BitOr::bitor,
        &BitOr::bitor,
        &|b, _| b,
    )
}
