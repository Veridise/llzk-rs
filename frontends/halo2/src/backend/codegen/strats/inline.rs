use crate::backend::codegen::inter_region_constraints;
use crate::backend::resolvers::ResolversProvider;
use crate::ir::chain_lowerable_stmts;
use crate::lookups::callbacks::LookupCallbacks;
use crate::synthesis::regions::{RegionRow, RegionRowLike as _, Row};
use crate::{
    backend::{
        codegen::{
            lookup::codegen::{InvokeLookupAsModule, LookupCodegenStrategy},
            lower_constraints, Codegen, CodegenStrategy,
        },
        lowering::Lowering,
    },
    synthesis::CircuitSynthesis,
};
use anyhow::Result;

#[derive(Default)]
pub struct InlineConstraintsStrat<LookupStrat: Default = InvokeLookupAsModule> {
    lookups: LookupStrat,
}

impl<LS> CodegenStrategy for InlineConstraintsStrat<LS>
where
    LS: Default + LookupCodegenStrategy,
{
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
        self.lookups.define_modules(codegen, syn, lookups)?;

        codegen.within_main(syn, move |_| {
            let lookups = self
                .lookups
                .invoke_lookups(syn, lookups)
                .and_then(|l| l.collect::<Result<Vec<_>>>())?;
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
                lookups,
                inter_region_constraints(syn)?
            ))
        })
    }
}
