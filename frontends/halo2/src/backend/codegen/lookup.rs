use std::{convert::identity, ops::BitOr};

use crate::{
    backend::{
        lowering::{Lowerable, Lowering, LoweringOutput},
        resolvers::ResolversProvider,
    },
    halo2::{Expression, Field, FixedQuery},
    ir::{expr::IRExpr, stmt::chain_lowerable_stmts, stmt::IRStmt},
    lookups::{callbacks::LookupCallbacks, Lookup},
    synthesis::{regions::RegionRowLike, CircuitSynthesis},
};
use anyhow::{anyhow, Result};

use super::Codegen;

//pub mod codegen;

#[inline]
fn zip_res<L, R, E>(lhs: Result<L, E>, rhs: Result<R, E>) -> Result<(L, R), E> {
    lhs.and_then(|lhs| rhs.map(|rhs| (lhs, rhs)))
}

pub fn codegen_lookup_modules<'c, C>(
    codegen: &C,
    syn: &CircuitSynthesis<C::F>,
    callbacks: &dyn LookupCallbacks<C::F>,
) -> Result<()>
where
    C: Codegen<'c>,
{
    // WIP
    for (kind, lookups) in syn.lookup_kinds()? {
        let io = kind
            .columns()
            .iter()
            .copied()
            .filter_map(|col| {
                lookups
                    .iter()
                    .filter_map(|l| {
                        l.expr_for_column(col)
                            .map(|e| callbacks.assign_io_kind(e, col).map(|io| (col, io)))
                            .transpose()
                    })
                    .reduce(|lhs, rhs| {
                        zip_res(lhs, rhs).and_then(|(lhs, rhs)| {
                            if lhs == rhs {
                                anyhow::bail!("Column {col} assigned different IO types")
                            }
                            Ok(lhs)
                        })
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        if let Some(module) = callbacks.on_body(&kind, &io.into_iter())? {
            module.generate(codegen)?;
        }
        //lookups.on_body(&kind, );
        //codegen.define_function_with_body(
        //    &kind.module_name(),
        //    kind.inputs(),
        //    kind.outputs(),
        //    syn,
        //    |_, inputs, outputs| {
        //        struct Dummy<F>(F);
        //
        //        impl<F: Field> Lowerable for Dummy<F> {
        //            type F = F;
        //
        //            fn lower<L>(self, _l: &L) -> Result<impl Into<LoweringOutput<L>>>
        //            where
        //                L: Lowering<F = Self::F> + ?Sized,
        //            {
        //                anyhow::bail!("Dummy value should not be lowered");
        //                #[allow(unreachable_code)]
        //                Ok(())
        //            }
        //        }
        //        Ok(chain_lowerable_stmts!(
        //            outputs
        //                .into_iter()
        //                .copied()
        //                .map(IRStmt::<Dummy<C::F>>::assume_deterministic),
        //            lookups.on_body(&kind, inputs, outputs)?
        //        )
        //        .collect())
        //    },
        //)?
    }
    Ok(())
}

fn comment<'r, T, F: Field>(lookup: Lookup<'r, F>, r: T) -> IRStmt<IRExpr<F>>
where
    T: ResolversProvider<F> + RegionRowLike + Copy + 'r,
{
    IRStmt::comment(format!(
        "Lookup {} '{}' @ region {} '{}' @ row {}",
        lookup.idx(),
        lookup.name(),
        r.region_index()
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "<unk>".to_string()),
        r.region_name(),
        r.row_number()
    ))
}

pub fn codegen_lookup_invocations<'s, F: Field>(
    syn: &'s CircuitSynthesis<F>,
    lookups: &'s dyn LookupCallbacks<F>,
) -> Result<Vec<IRStmt<IRExpr<F>>>> {
    syn.lookups_per_region_row()
        .map(|(r, l)| {
            syn.tables_for_lookup(&l)
                .and_then(|table| lookups.on_lookup(&r, l, &table))
                .map(|stmts| {
                    chain_lowerable_stmts!([comment(l, r)], stmts)
                        .collect::<IRStmt<_>>()
                        .map(&|t| t.unwrap())
                })
        })
        .collect()
}

pub fn query_from_table_expr<F: Field>(e: &Expression<F>) -> Result<FixedQuery> {
    match e {
        Expression::Fixed(fixed_query) => Ok(*fixed_query),
        _ => Err(anyhow!(
            "Table row expressions can only be fixed cell queries"
        )),
    }
}

pub fn contains_fixed<F: Field>(e: &&Expression<F>) -> bool {
    fn false_cb<I>(_: I) -> bool {
        false
    }
    e.evaluate(
        &false_cb,
        &false_cb,
        &|_| true,
        &false_cb,
        &false_cb,
        &false_cb,
        &identity,
        &BitOr::bitor,
        &BitOr::bitor,
        &|b, _| b,
    )
}
