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

use crate::backend::codegen::lookup::Lookup;

use super::LookupCodegenStrategy;

#[derive(Default)]
pub struct LookupAsRowConstraint;

impl LookupCodegenStrategy for LookupAsRowConstraint {
    fn define_modules<'c, C>(&self, _codegen: &C, _syn: &CircuitSynthesis<C::F>) -> Result<()>
    where
        C: Codegen<'c>,
    {
        Ok(())
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
        let tables = syn.tables();

        fn tables_for_lookup<F: crate::halo2::Field>(
            tables: &[TableData<F>],
            l: &Lookup<F>,
        ) -> Result<Vec<Vec<Value<F>>>> {
            // 1. Load the table we are looking up
            // For each table region look if they have the columns we are looking for and
            // collect all the fixed values
            tables
                .iter()
                .map(|table| {
                    let q = l.output_queries();
                    table.get_rows(q).and_then(|t| {
                    if q.len() !=  t.len() {
                        bail!("Inconsistency check failed: Lookup has {} columns but table yielded {}", q.len(), t.len())
                    }
                    Ok(t)})
                })
                .reduce(|acc, t| acc.or_else(|_| t))
                .ok_or_else(|| anyhow!("Could not get values from table"))
                .and_then(identity).map(transpose)
        }

        Lookup::load(syn).and_then(|lookups| {
            lookups
                .into_iter()
                .map(move |l| {
                    let values = tables_for_lookup(tables, &l)?
                        .into_iter()
                        .map(|vals| {
                            assert!(
                                l.inputs.len() == vals.len(),
                                "inputs({}) = {:?} | vals({}) = {vals:?}",
                                l.inputs.len(),
                                l.inputs,
                                vals.len()
                            );
                            vals.into_iter().zip(l.inputs).collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>();

                    Ok(region_rows()
                        .map(move |row| {
                            values
                                .iter()
                                .map(|constraint| {
                                    lower_constraint_parts(row, constraint.as_slice(), scope)
                                })
                                .reduce(|lhs, rhs| scope.lower_or(&lhs?, &rhs?))
                                .unwrap()
                        })
                        .map(|e| e.map(CircuitStmt::Assert)))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|stmts| stmts.into_iter().flatten())
        })
    }
}

fn lower_constraint_parts<L>(
    row: RegionRow<L::F>,
    constraint: &[(Value<L::F>, &Expression<L::F>)],
    scope: &L,
) -> Result<L::CellOutput>
where
    L: Lowering,
{
    constraint
        .iter()
        .map(|(v, e)| {
            let v = steal(v).ok_or_else(|| anyhow!("Table value was unknown!"))?;
            let v = scope.lower_constant(v)?;
            let e = scope.lower_expr(e, &row, &row)?;

            scope.lower_eq(&v, &e)
        })
        .reduce(|lhs, rhs| scope.lower_and(&lhs?, &rhs?))
        .unwrap()
}

fn transpose<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    assert!(!v.is_empty());
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| n.next().unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}
