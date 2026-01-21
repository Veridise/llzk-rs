//! Structs for handling calls between groups.

use crate::{
    expressions::ScopedExpression,
    synthesis::{
        groups::{Group, GroupCell, GroupKey},
        regions::{RegionData, RegionRow, Row},
    },
    temps::ExprOrTemp,
};

use anyhow::Result;
use eqv::EqvRelation;
use ff::Field;
use halo2_frontend_core::expressions::ExprBuilder;
use haloumi_ir::{
    CmpOp, Felt, Slot, SymbolicEqv, expr::IRAexpr, groups::callsite::CallSite, stmt::IRStmt,
    traits::ConstantFolding,
};
use haloumi_lowering::{
    Lowering,
    lowerable::{LowerableExpr, LowerableStmt},
};

///// Data related to a single callsite
//#[derive(Debug)]
//pub struct CallSite<E> {
//    name: String,
//    callee: GroupKey,
//    /// The index in the original groups array to the called group.
//    callee_id: usize,
//    inputs: Vec<E>,
//    output_vars: Vec<FuncIO>,
//    outputs: Vec<E>,
//}

fn cells_to_exprs<'e, 's, 'syn, 'cb, 'io, F, E>(
    cells: &[GroupCell],
    ctx: &super::GroupIRCtx<'cb, '_, 'syn, F, E>,
    advice_io: &'io crate::io::AdviceIO,
    instance_io: &'io crate::io::InstanceIO,
) -> anyhow::Result<Vec<ExprOrTemp<ScopedExpression<'e, 's, F, E>>>>
where
    'syn: 's,
    'io: 's,
    F: Field,
    E: Clone + ExprBuilder<F>,
{
    cells
        .iter()
        .map(|cell| {
            let region: Option<RegionData<'syn>> = cell
                .region_index()
                .map(|index| {
                    ctx.regions_by_index().get(&index).ok_or_else(|| {
                        anyhow::anyhow!("Region with index {} is not a known region", *index)
                    })
                })
                .transpose()?
                .copied();

            let expr = cell.to_expr::<F, E>();
            let row = match cell {
                GroupCell::Assigned(cell) => {
                    let start = ctx.regions_by_index()[&cell.region_index.into()]
                        .start()
                        .ok_or_else(|| {
                            anyhow::anyhow!("Region {} does not have a start", *cell.region_index)
                        })?;
                    cell.row_offset + start
                }
                GroupCell::InstanceIO((_, row)) => *row,
                GroupCell::AdviceIO((_, row)) => *row,
            };
            log::debug!(
                "Lowering cell {cell:?} (We have region? {})",
                region.is_some()
            );
            Ok(match region {
                Some(region) => ScopedExpression::new(
                    expr,
                    RegionRow::new(
                        region,
                        row,
                        advice_io,
                        instance_io,
                        ctx.syn().fixed_query_resolver(),
                    ),
                ),
                None => ScopedExpression::new(
                    expr,
                    Row::new(
                        row,
                        advice_io,
                        instance_io,
                        ctx.syn().fixed_query_resolver(),
                    ),
                ),
            })
        })
        .map(|e| e.map(ExprOrTemp::Expr))
        .collect()
}

//impl<'s, 'syn, 'ctx, F, E> CallSite<ExprOrTemp<ScopedExpression<'_, 's, F, E>>>
//where
//    'syn: 's,
//    'ctx: 's,
//    F: Field,
//    E: Clone + ExprBuilder<F>,
//{
pub(super) fn new_callsite<'s, 'e, 'syn, 'ctx, F, E>(
    callee: &Group,
    callee_id: usize,
    ctx: &super::GroupIRCtx<'_, '_, 'syn, F, E>,
    call_no: usize,
    advice_io: &'ctx crate::io::AdviceIO,
    instance_io: &'ctx crate::io::InstanceIO,
) -> anyhow::Result<CallSite<ExprOrTemp<ScopedExpression<'e, 's, F, E>>>>
where
    'syn: 's,
    'ctx: 's,
    F: Field,
    E: Clone + ExprBuilder<F>,
{
    let callee_key = callee
        .key()
        .ok_or_else(|| anyhow::anyhow!("Top level cannot be called by other group"))?;

    let inputs = cells_to_exprs(callee.inputs(), ctx, advice_io, instance_io)?;
    let outputs = cells_to_exprs(callee.outputs(), ctx, advice_io, instance_io)?;
    let output_vars: Vec<_> = callee
        .outputs()
        .iter()
        .enumerate()
        .map(|(n, _)| Slot::CallOutput(call_no, n))
        .collect();

    Ok(CallSite::new(
        callee.name().to_owned(),
        callee_key,
        callee_id,
        inputs,
        output_vars,
        outputs,
    ))
}

//impl<E> CallSite<E> {
//    /// Returns the index in the groups list of the called group
//    pub fn callee_id(&self) -> usize {
//        self.callee_id
//    }
//
//    /// Sets the callee id.
//    pub fn set_callee_id(&mut self, callee_id: usize) {
//        self.callee_id = callee_id;
//    }
//
//    /// Returns the name of the callee.
//    pub fn name(&self) -> &str {
//        &self.name
//    }
//
//    /// Sets the name of the called group.
//    pub fn set_name(&mut self, name: String) {
//        self.name = name;
//    }
//
//    /// Tries to transform the inner expression type into another.
//    pub fn try_map<O>(self, f: &impl Fn(E) -> Result<O>) -> Result<CallSite<O>> {
//        Ok(CallSite {
//            name: self.name,
//            callee: self.callee,
//            callee_id: self.callee_id,
//            inputs: self
//                .inputs
//                .into_iter()
//                .map(f)
//                .collect::<Result<Vec<_>, _>>()?,
//            output_vars: self.output_vars,
//            outputs: self
//                .outputs
//                .into_iter()
//                .map(f)
//                .collect::<Result<Vec<_>, _>>()?,
//        })
//    }
//
//    /// Returns the inputs of the call.
//    pub fn inputs(&self) -> &[E] {
//        &self.inputs
//    }
//
//    /// Returns the names of the outputs of the call.
//    pub fn output_vars(&self) -> &[FuncIO] {
//        &self.output_vars
//    }
//
//    /// Returns the outputs of the call.
//    pub fn outputs(&self) -> &[E] {
//        &self.outputs
//    }
//}
