use crate::halo2::{Expression, Field};
use anyhow::{Result, anyhow};
use halo2_proofs::plonk::FixedQuery;

pub fn query_from_table_expr<F: Field>(e: &Expression<F>) -> Result<FixedQuery> {
    match e {
        Expression::Fixed(fixed_query) => Ok(*fixed_query),
        _ => Err(anyhow!(
            "Table row expressions can only be fixed cell queries"
        )),
    }
}
