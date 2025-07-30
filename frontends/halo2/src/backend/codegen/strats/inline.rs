use crate::{
    backend::{
        codegen::{
            lookup::codegen::{InvokeLookupAsModule, LookupCodegenStrategy},
            Codegen, CodegenStrategy,
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
    fn codegen<'c, C>(&self, codegen: &C, syn: &CircuitSynthesis<C::F>) -> Result<()>
    where
        C: Codegen<'c>,
    {
        self.lookups.define_modules(codegen, syn)?;

        codegen.within_main(syn, move |scope| {
            let lookups = self.lookups.invoke_lookups(scope, syn)?;
            // Do the region stmts first since backends may have more information about names for
            // cells there and some backends do not update the name and always use the first
            // one given.
            syn.region_gates()
                .flat_map(|(gate, r)| {
                    scope.lower_constraints(gate, r, r.header(), Some(r.row_number()))
                })
                .chain(lookups)
                .chain(self.inter_region_constraints(scope, syn))
                .collect::<Result<Vec<_>>>()
        })
    }
}
