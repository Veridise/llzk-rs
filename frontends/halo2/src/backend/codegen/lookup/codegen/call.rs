use anyhow::Result;

use crate::{
    backend::{
        codegen::{
            lookup::{contains_fixed, query_from_table_expr},
            Codegen,
        },
        func::FuncIO,
        lowering::{Lowerable, LowerableOrIO, Lowering, LoweringOutput},
        resolvers::ResolversProvider,
    },
    expressions::{ExpressionFactory as _, ScopedExpression},
    halo2::{Expression, Field},
    ir::chain_lowerable_stmts,
    lookups::{callbacks::LookupCallbacks, Lookup},
    synthesis::{regions::RegionRowLike, CircuitSynthesis},
    BinaryBoolOp, CircuitStmt,
};

use super::LookupCodegenStrategy;

#[derive(Default)]
pub struct InvokeLookupAsModule {}

impl LookupCodegenStrategy for InvokeLookupAsModule {
    fn define_modules<'c, C>(
        &self,
        codegen: &C,
        syn: &CircuitSynthesis<C::F>,
        lookups: &dyn LookupCallbacks<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c>,
    {
        for kind in syn.lookup_kinds(lookups)? {
            codegen.define_function_with_body(
                &kind.module_name(),
                kind.inputs(),
                kind.outputs(),
                syn,
                |_, inputs, outputs| {
                    struct Dummy<F>(F);

                    impl<F: Field> Lowerable for Dummy<F> {
                        type F = F;

                        fn lower<L>(self, _l: &L) -> Result<impl Into<LoweringOutput<L>>>
                        where
                            L: Lowering<F = Self::F> + ?Sized,
                        {
                            anyhow::bail!("Dummy value should not be lowered");
                            #[allow(unreachable_code)]
                            Ok(())
                        }
                    }
                    Ok(chain_lowerable_stmts!(
                        outputs
                            .into_iter()
                            .copied()
                            .map(CircuitStmt::<Dummy<C::F>>::assume_deterministic),
                        lookups.on_body(&kind, inputs, outputs)?
                    )
                    .collect())
                },
            )?
        }
        Ok(())
    }

    fn invoke_lookups<'s, F: Field>(
        &self,
        syn: &'s CircuitSynthesis<F>,
        lookups: &'s dyn LookupCallbacks<F>,
    ) -> Result<impl Iterator<Item = Result<CircuitStmt<impl Lowerable<F = F> + 's>>> + 's> {
        Ok(syn.lookups_per_region_row(lookups).map(|(r, l)| {
            let additional = l.callbacks().on_call(&r, l)?;
            Ok(chain_lowerable_stmts!(create_call_stmt(l, r), additional)
                .collect::<CircuitStmt<_>>())
        }))
    }
}

type TableLookup<'a, F> = (&'a Expression<F>, &'a Expression<F>);

fn get_lookup_io<'a, F: Field>(
    lookup: Lookup<'a, F>,
) -> (Vec<TableLookup<'a, F>>, Vec<TableLookup<'a, F>>) {
    lookup
        .expressions()
        .partition::<Vec<_>, _>(|(e, _)| contains_fixed(e))
}

fn process_outputs<'a, 'r, I, T, F: Field>(
    lookup_id: u64,
    lookup_idx: usize,
    outputs: I,
    region_row: T,
) -> Result<(
    Vec<FuncIO>,
    Vec<CircuitStmt<LowerableOrIO<ScopedExpression<'a, 'r, F>>>>,
)>
where
    I: Iterator<Item = TableLookup<'a, F>>,
    T: ResolversProvider<F> + RegionRowLike + Copy + 'r,
{
    let row = region_row.row_number();
    outputs
        .into_iter()
        .map(move |(e, o)| {
            let o = query_from_table_expr(o).and_then(|q| {
                Ok(FuncIO::TableLookup(
                    lookup_id,
                    q.column_index(),
                    row,
                    lookup_idx,
                    region_row
                        .region_index()
                        .ok_or_else(|| anyhow::anyhow!("No region index"))?,
                ))
            })?;
            let e = LowerableOrIO::from(region_row.create_ref(e));
            let oe = LowerableOrIO::from(o);
            Ok((o, CircuitStmt::constraint(BinaryBoolOp::Eq, oe, e)))
        })
        .collect::<Result<Vec<_>>>()
        .map(move |v| v.into_iter().unzip())
}

fn process_inputs<'r, I, T, F>(
    inputs: I,
    r: T,
) -> impl Iterator<Item = ScopedExpression<'r, 'r, F>> + 'r
where
    I: Iterator<Item = TableLookup<'r, F>> + 'r,
    T: ResolversProvider<F> + Copy + 'r,
    F: Field,
{
    inputs.into_iter().map(move |(e, _)| r.create_ref(e))
}

fn create_call_stmt<'r, T, F: Field>(
    lookup: Lookup<'r, F>,
    r: T,
) -> Result<CircuitStmt<impl Lowerable<F = F> + 'r>>
where
    T: ResolversProvider<F> + RegionRowLike + Copy + 'r,
{
    let lookup_id = lookup.kind()?.id();
    let lookup_idx = lookup.idx();
    let (inputs, outputs) = get_lookup_io(lookup);
    let (vars, constraints) = process_outputs(lookup_id, lookup_idx, outputs.into_iter(), r)?;

    let call = CircuitStmt::call(
        lookup.module_name()?,
        process_inputs(inputs.into_iter(), r),
        vars,
    );

    let comment = CircuitStmt::comment(format!(
        "Lookup {} '{}' @ region {} '{}' @ row {}",
        lookup.idx(),
        lookup.name(),
        r.region_index()
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "<unk>".to_string()),
        r.region_name(),
        r.row_number()
    ));
    Ok(chain_lowerable_stmts!([comment, call], constraints)
        .map(|s| s.map(&|l| l.fold_right()))
        .collect::<CircuitStmt<_>>())
}
