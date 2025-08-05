use anyhow::Result;

use crate::{
    backend::{
        codegen::Codegen,
        lowering::{Lowerable, Lowering},
        resolvers::ResolversProvider,
    },
    halo2::Field,
    lookups::callbacks::LookupCallbacks,
    synthesis::{
        regions::{RegionRow, Row},
        CircuitSynthesis,
    },
    CircuitStmt,
};

mod call;
//mod inlined;

pub use call::InvokeLookupAsModule;
//pub use inlined::LookupAsRowConstraint;

pub trait LookupCodegenStrategy {
    fn define_modules<'c, C>(
        &self,
        codegen: &C,
        syn: &CircuitSynthesis<C::F>,
        lookups: &dyn LookupCallbacks<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c>;

    fn invoke_lookups<'s, F: Field>(
        &self,
        syn: &'s CircuitSynthesis<F>,
        lookups: &'s dyn LookupCallbacks<F>,
    ) -> Result<impl Iterator<Item = Result<CircuitStmt<impl Lowerable<F = F> + 's>>> + 's>;
}
