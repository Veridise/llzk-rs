use std::{convert::identity, iter};

use anyhow::{anyhow, bail, Result};

use crate::{
    backend::{
        codegen::Codegen,
        lowering::Lowering,
        resolvers::{QueryResolver as _, ResolvedQuery},
    },
    halo2::{Expression, Value},
    synthesis::{
        regions::{RegionRow, TableData},
        CircuitSynthesis,
    },
    value::steal,
    BinaryBoolOp, CircuitStmt,
};

use super::Lookup;

mod call;
mod inlined;

pub use call::InvokeLookupAsModule;
pub use inlined::LookupAsRowConstraint;

pub trait LookupCodegenStrategy {
    fn define_modules<'c, C>(&self, codegen: &C, syn: &CircuitSynthesis<C::F>) -> Result<()>
    where
        C: Codegen<'c>;

    fn invoke_lookups<L>(
        &self,
        scope: &L,
        syn: &CircuitSynthesis<L::F>,
    ) -> Result<impl Iterator<Item = Result<CircuitStmt<L::CellOutput>>>>
    where
        L: Lowering;
}
