use std::borrow::Cow;

use crate::{
    backend::codegen::{Codegen, CodegenStrategy},
    io::AllCircuitIO,
    ir::{expr::IRAexpr, IRCircuit, IRCtx},
};
use anyhow::Result;

/// Code generation strategy that generates the all the code inside the main function.
#[derive(Default)]
#[allow(dead_code)]
pub struct InlineConstraintsStrat {}

impl CodegenStrategy for InlineConstraintsStrat {
    fn codegen<'c: 'st, 's, 'st, C>(
        &self,
        codegen: &C,
        ctx: &IRCtx,
        ir: &IRCircuit<IRAexpr>,
        //lookups: &dyn LookupCallbacks<C::F>,
        //gate_cbs: &dyn GateCallbacks<C::F>,
        //injector: &mut dyn crate::IRInjectCallback<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c, 'st>,
        //Row<'s, 's, C::F>: ResolversProvider<C::F> + 's,
        //RegionRow<'s, 's, 's, C::F>: ResolversProvider<C::F> + 's,
    {
        log::debug!(
            "Performing codegen with {} strategy",
            std::any::type_name_of_val(self)
        );

        log::debug!("Generating main body");
        let io = AllCircuitIO {
            advice: Cow::Borrowed(ctx.advice_io_of_group(ir.main().id())),
            instance: Cow::Borrowed(ctx.instance_io_of_group(ir.main().id())),
        };
        codegen.define_main_function_with_body(io, ir.groups().to_vec())

        //codegen.within_main(ir.main().all_io(), move |_| {
        //    Ok([ir.groups().to_vec()])
        //let patterns = load_patterns(gate_cbs);
        //
        //let mut stmts: Vec<IRStmt<_>> = vec![];
        //let top_level = syn
        //    .top_level_group()
        //    .ok_or_else(|| anyhow::anyhow!("Circuit synthesis is missing a top level group"))?;
        //let advice_io = top_level.advice_io();
        //let instance_io = top_level.instance_io();
        //for group in syn.groups() {
        //    // Do the region stmts first since backends may have more information about names for
        //    // cells there and some backends do not update the name and always use the first
        //    // one given.
        //
        //    log::debug!("Lowering gates");
        //    stmts.extend(
        //        lower_gates(
        //            syn.gates(),
        //            &group.regions(),
        //            &patterns,
        //            advice_io,
        //            instance_io,
        //            syn.fixed_query_resolver(),
        //        )
        //        .and_then(scoped_exprs_to_aexpr)?,
        //    );
        //    log::debug!("Lowering lookups");
        //    stmts.extend(
        //        codegen_lookup_invocations(
        //            syn,
        //            group.region_rows(syn.fixed_query_resolver()).as_slice(),
        //            lookups,
        //        )
        //        .and_then(scoped_exprs_to_aexpr)?,
        //    );
        //    log::debug!("Lowering inter region equality constraints");
        //    stmts.extend(scoped_exprs_to_aexpr(inter_region_constraints(
        //        syn.constraints().edges(),
        //        advice_io,
        //        instance_io,
        //        syn.fixed_query_resolver(),
        //    )));
        //
        //    for region in group.regions() {
        //        let index = region
        //            .index()
        //            .ok_or_else(|| anyhow::anyhow!("Region does not have an index"))?;
        //        let start = region.start().unwrap_or_default();
        //        if let Some(ir) = injector.inject(index, start) {
        //            stmts.push(crate::backend::codegen::lower_injected_ir(
        //                ir,
        //                region,
        //                advice_io,
        //                instance_io,
        //                syn.fixed_query_resolver(),
        //            )?);
        //        }
        //    }
        //}
        //Ok(stmts)
    }
}
