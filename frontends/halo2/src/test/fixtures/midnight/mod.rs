use std::borrow::Cow;

use midnight_halo2_proofs::plonk::Expression;

use crate::{
    halo2::Field,
    ir::stmt::IRStmt,
    lookups::{
        callbacks::{LookupCallbacks, LookupTableGenerator},
        Lookup,
    },
};

pub mod fibonacci;
pub mod lookup;
pub mod lookup_2x3;
pub mod lookup_2x3_fixed;
pub mod lookup_2x3_zerosel;
pub mod mul;
pub mod mul_with_fixed_constraint;

struct LookupCallbackHandler;

impl<F: Field> LookupCallbacks<F> for LookupCallbackHandler {
    fn on_lookup<'a>(
        &self,
        lookup: Lookup<'a, F>,
        table: &dyn LookupTableGenerator<F>,
    ) -> anyhow::Result<IRStmt<Cow<'a, Expression<F>>>> {
        Ok(IRStmt::comment("Ignored lookup"))
    }
}
