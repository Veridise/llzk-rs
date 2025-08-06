use crate::{
    backend::{
        codegen::{
            inter_region_constraints,
            lookup::{codegen_lookup_invocations, codegen_lookup_modules},
            lower_constraints, Codegen, CodegenStrategy,
        },
        resolvers::ResolversProvider,
    },
    ir::stmt::chain_lowerable_stmts,
    lookups::callbacks::LookupCallbacks,
    synthesis::{
        regions::{RegionRow, RegionRowLike as _, Row},
        CircuitSynthesis,
    },
};
use anyhow::Result;

#[derive(Default)]
pub struct InlineConstraintsStrat {}

impl CodegenStrategy for InlineConstraintsStrat {
    fn codegen<'c, 's, C>(
        &self,
        codegen: &C,
        syn: &'s CircuitSynthesis<C::F>,
        lookups: &dyn LookupCallbacks<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c>,
        Row<'s, C::F>: ResolversProvider<C::F> + 's,
        RegionRow<'s, 's, C::F>: ResolversProvider<C::F> + 's,
    {
        //self.lookups.define_modules(codegen, syn, lookups)?;
        codegen_lookup_modules(codegen, syn, lookups)?;

        codegen.within_main(syn, move |_| {
            // Do the region stmts first since backends may have more information about names for
            // cells there and some backends do not update the name and always use the first
            // one given.
            Ok(chain_lowerable_stmts!(
                syn.region_gates().flat_map(|(gate, r)| lower_constraints(
                    gate,
                    r,
                    r.header(),
                    Some(r.row_number())
                )),
                codegen_lookup_invocations(syn, lookups)?,
                inter_region_constraints(syn)?
            ))
        })
    }
}
