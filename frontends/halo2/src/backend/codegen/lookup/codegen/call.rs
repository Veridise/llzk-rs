use std::collections::HashSet;

use anyhow::Result;

use crate::{
    backend::{
        codegen::{
            lookup::{contains_fixed, query_from_table_expr, Lookup},
            Codegen,
        },
        func::FuncIO,
        lowering::Lowering,
    },
    halo2::{Expression, Field},
    synthesis::{regions::RegionRow, CircuitSynthesis},
    BinaryBoolOp, CircuitStmt,
};

use super::LookupCodegenStrategy;

#[derive(Default)]
pub struct InvokeLookupAsModule {}

impl LookupCodegenStrategy for InvokeLookupAsModule {
    fn define_modules<'c, C>(&self, codegen: &C, syn: &CircuitSynthesis<C::F>) -> Result<()>
    where
        C: Codegen<'c>,
    {
        Lookup::load(syn)?
            .iter()
            .map(|l| l.kind())
            .collect::<HashSet<_>>()
            .into_iter()
            .try_for_each(|kind| {
                codegen.define_function_with_body(
                    &kind.module_name(),
                    kind.inputs(),
                    kind.outputs(),
                    syn,
                    |_, _, outputs| {
                        Ok(outputs
                            .into_iter()
                            .copied()
                            .map(CircuitStmt::AssumeDeterministic)
                            .collect())
                    },
                )
            })
    }

    fn invoke_lookups<L>(
        &self,
        scope: &L,
        syn: &CircuitSynthesis<L::F>,
    ) -> Result<impl Iterator<Item = Result<CircuitStmt<L::CellOutput>>>>
    where
        L: Lowering,
    {
        let region_rows = move || {
            syn.regions().into_iter().flat_map(move |r| {
                r.rows()
                    .map(move |row| RegionRow::new(r, row, syn.advice_io(), syn.instance_io()))
            })
        };
        Ok(Lookup::load(syn)?
            .into_iter()
            .flat_map(move |l| region_rows().map(move |r| create_call_stmt(&l, scope, &r))))
    }
}

type TableLookup<'a, F> = (&'a Expression<F>, &'a Expression<F>);

fn get_lookup_io<'a, F: Field>(
    lookup: &'a Lookup<'a, F>,
) -> (Vec<TableLookup<'a, F>>, Vec<TableLookup<'a, F>>) {
    lookup
        .expressions()
        .partition::<Vec<_>, _>(|(e, _)| contains_fixed(e))
}

fn process_inputs<L>(
    inputs: Vec<TableLookup<L::F>>,
    scope: &L,
    r: &RegionRow<L::F>,
) -> Result<Vec<L::CellOutput>>
where
    L: Lowering,
{
    let inputs = inputs.into_iter().map(|(e, _)| e).collect::<Vec<_>>();
    scope.lower_expr_refs(inputs.as_slice(), r, r)
}

fn process_outputs<L>(
    lookup: &Lookup<L::F>,
    outputs: Vec<TableLookup<L::F>>,
    scope: &L,
    r: &RegionRow<L::F>,
) -> Result<(Vec<FuncIO>, Vec<CircuitStmt<L::CellOutput>>)>
where
    L: Lowering,
{
    let lookup_id = lookup.kind().id();
    let row = r.row_number();
    outputs
        .into_iter()
        .map(|(e, o)| {
            let o = query_from_table_expr(o)
                .map(|q| FuncIO::TableLookup(lookup_id, q.column_index(), row, lookup.idx))?;
            let e = scope.lower_expr(e, r, r)?;
            let oe = scope.lower_funcio(o)?;
            Ok((o, CircuitStmt::Constraint(BinaryBoolOp::Eq, e, oe)))
        })
        .collect::<Result<Vec<_>>>()
        .map(|v| v.into_iter().unzip())
}

fn create_call_stmt<L>(
    lookup: &Lookup<L::F>,
    scope: &L,
    r: &RegionRow<L::F>,
) -> Result<CircuitStmt<L::CellOutput>>
where
    L: Lowering,
{
    let (inputs, outputs) = get_lookup_io(lookup);
    let (vars, constraints) = process_outputs(lookup, outputs, scope, r)?;

    Ok(CircuitStmt::Seq(
        [
            CircuitStmt::Comment(format!(
                "Lookup {} '{}' @ row {}",
                lookup.idx(),
                lookup.name(),
                r.row_number()
            )),
            CircuitStmt::ConstraintCall(
                lookup.module_name(),
                process_inputs(inputs, scope, r)?,
                vars,
            ),
        ]
        .into_iter()
        .chain(constraints)
        .collect::<Vec<_>>(),
    ))
}
