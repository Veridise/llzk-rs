use crate::{
    backend::{
        func::FuncIO,
        lowering::{
            lowerable::{LowerableExpr, LowerableStmt},
            Lowering,
        },
    },
    expressions::ScopedExpression,
    halo2::{groups::GroupKeyInstance, Advice, Field, Instance},
    ir::{
        equivalency::{EqvRelation, SymbolicEqv},
        expr::IRAexpr,
        stmt::IRStmt,
        CmpOp,
    },
    synthesis::{
        groups::{Group, GroupCell},
        regions::{RegionData, RegionRow, Row},
    },
    CircuitIO,
};
use anyhow::Result;

/// Data related to a single callsite
#[derive(Debug)]
pub struct CallSite<E> {
    name: String,
    callee: GroupKeyInstance,
    /// The index in the original groups array to the called group.
    callee_id: usize,
    inputs: Vec<E>,
    output_vars: Vec<FuncIO>,
    outputs: Vec<E>,
}

fn cells_to_exprs<'e, 's, F: Field>(
    cells: &[GroupCell],
    ctx: &super::GroupIRCtx<'_, 's, F>,
    advice_io: &'s CircuitIO<Advice>,
    instance_io: &'s CircuitIO<Instance>,
) -> anyhow::Result<Vec<ScopedExpression<'e, 's, F>>> {
    cells
        .iter()
        .map(|cell| {
            let region: Option<RegionData<'_>> = cell
                .region_index()
                .map(|index| {
                    ctx.regions_by_index().get(&index).ok_or_else(|| {
                        anyhow::anyhow!("Region with index {} is not a known region", *index)
                    })
                })
                .transpose()?
                .copied();

            let expr = cell.to_expr::<F>();
            let row = match cell {
                GroupCell::Assigned(cell) => {
                    let start = ctx.regions_by_index()[&cell.region_index]
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
            //.try_into()
        })
        .collect()
}

impl EqvRelation<CallSite<IRAexpr>> for SymbolicEqv {
    /// Two callsites are equivalent if the call statement they represent is equivalent.
    fn equivalent(lhs: &CallSite<IRAexpr>, rhs: &CallSite<IRAexpr>) -> bool {
        lhs.callee == rhs.callee
            && Self::equivalent(&lhs.inputs, &rhs.inputs)
            && Self::equivalent(&lhs.outputs, &rhs.outputs)
    }
}

impl<'s, F: Field> CallSite<ScopedExpression<'_, 's, F>> {
    pub(super) fn new(
        callee: &Group,
        callee_id: usize,
        ctx: &super::GroupIRCtx<'_, 's, F>,
        call_no: usize,
        advice_io: &'s CircuitIO<Advice>,
        instance_io: &'s CircuitIO<Instance>,
        free_cells: &[GroupCell],
    ) -> anyhow::Result<Self> {
        let callee_key = callee
            .key()
            .ok_or_else(|| anyhow::anyhow!("Top level cannot be called by other group"))?;

        let inputs = cells_to_exprs(
            &[callee.inputs(), free_cells].concat(),
            ctx,
            advice_io,
            instance_io,
        )?;
        let outputs = cells_to_exprs(callee.outputs(), ctx, advice_io, instance_io)?;
        let output_vars: Vec<_> = callee
            .outputs()
            .iter()
            .enumerate()
            .map(|(n, _)| FuncIO::CallOutput(call_no, n))
            .collect();

        Ok(Self {
            name: callee.name().to_owned(),
            callee: callee_key,
            inputs,
            output_vars,
            outputs,
            callee_id,
        })
    }
}

impl<E> CallSite<E> {
    pub fn callee_id(&self) -> usize {
        self.callee_id
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn try_map<O>(self, f: &impl Fn(E) -> Result<O>) -> Result<CallSite<O>> {
        Ok(CallSite {
            name: self.name,
            callee: self.callee,
            callee_id: self.callee_id,
            inputs: self
                .inputs
                .into_iter()
                .map(|e| f(e))
                .collect::<Result<Vec<_>, _>>()?,
            output_vars: self.output_vars,
            outputs: self
                .outputs
                .into_iter()
                .map(|e| f(e))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl LowerableStmt for CallSite<IRAexpr> {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        let inputs: Vec<_> = self
            .inputs
            .into_iter()
            .map(|e| e.lower(l))
            .collect::<Result<_, _>>()?;
        l.generate_call(self.name.as_str(), &inputs, &self.output_vars)?;
        // The call statement creates variables that we need to constraint against the actual
        // outputs.
        for (lhs, rhs) in
            std::iter::zip(self.outputs, self.output_vars.into_iter().map(IRAexpr::IO))
        {
            IRStmt::constraint(CmpOp::Eq, lhs, rhs).lower(l)?
        }
        Ok(())
    }
}

impl<E: Clone> Clone for CallSite<E> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            callee: self.callee.clone(),
            callee_id: self.callee_id.clone(),
            inputs: self.inputs.clone(),
            output_vars: self.output_vars.clone(),
            outputs: self.outputs.clone(),
        }
    }
}
