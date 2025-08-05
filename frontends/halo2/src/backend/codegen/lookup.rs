use std::{convert::identity, ops::BitOr};

use crate::halo2::{Expression, Field, FixedQuery};
use anyhow::{anyhow, Result};

pub mod codegen;

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
