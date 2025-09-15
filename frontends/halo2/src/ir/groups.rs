use crate::{
    backend::{
        codegen::{
            lookup::codegen_lookup_invocations,
            //scoped_exprs_to_aexpr,
            //strats::{
            //    groups::{
            //        bounds::{Bound, EqConstraintCheck, GroupBounds},
            //        callsite::CallSite,
            //        free_cells::FreeCells,
            //        GroupIRCtx,
            //    },
            //    lower_gates,
            //},
        },
        func::{try_relativize_advice_cell, FuncIO},
        lowering::{
            lowerable::{LowerableExpr, LowerableStmt},
            Lowering,
        },
        resolvers::FixedQueryResolver,
    },
    expressions::{ExpressionInRow, ScopedExpression},
    gates::RewritePatternSet,
    halo2::{
        groups::GroupKeyInstance, Advice, Expression, Field, Gate, Instance, RegionIndex, Rotation,
    },
    io::AllCircuitIO,
    ir::{
        equivalency::{EqvRelation, SymbolicEqv},
        expr::IRAexpr,
        generate::{free_cells::FreeCells, GroupIRCtx},
        groups::{
            bounds::{Bound, EqConstraintCheck, GroupBounds},
            callsite::CallSite,
        },
        stmt::IRStmt,
        CmpOp, IRCtx,
    },
    synthesis::{
        constraint::EqConstraint,
        groups::{Group, GroupCell},
        regions::{RegionData, Row},
    },
    utils, CircuitIO, GateRewritePattern as _, GateScope, RewriteError,
};
use anyhow::Result;
use std::{borrow::Cow, collections::HashMap};

pub mod bounds;
pub mod callsite;

/// Group's IR
#[derive(Debug)]
pub struct GroupBody<E> {
    name: String,
    /// Index in the original groups array.
    id: usize,
    input_count: usize,
    output_count: usize,
    key: Option<GroupKeyInstance>,
    gates: IRStmt<E>,
    eq_constraints: IRStmt<E>,
    callsites: Vec<CallSite<E>>,
    lookups: IRStmt<E>,
    injected: Vec<IRStmt<E>>,
}

impl<'cb, 's, F: Field> GroupBody<ScopedExpression<'s, 's, F>> {
    pub(super) fn new(
        group: &'s Group,
        id: usize,
        ctx: &GroupIRCtx<'cb, 's, F>,
        free_cells: &FreeCells,
        advice_io: &'s CircuitIO<Advice>,
        instance_io: &'s CircuitIO<Instance>,
    ) -> anyhow::Result<Self> {
        log::debug!("Lowering call-sites for group {:?}", group.name());
        let callsites = {
            group
                .children(ctx.groups())
                .into_iter()
                .enumerate()
                .map(|(call_no, (callee_id, callee))| {
                    CallSite::new(
                        callee,
                        callee_id,
                        ctx,
                        call_no,
                        advice_io,
                        instance_io,
                        &free_cells.callsites[call_no],
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        log::debug!("Lowering gates for group {:?}", group.name());
        let gates = IRStmt::seq(lower_gates(
            ctx.syn().gates(),
            &group.regions(),
            ctx.patterns(),
            &advice_io,
            &instance_io,
            ctx.syn().fixed_query_resolver(),
        )?);
        //.and_then(scoped_exprs_to_aexpr)?;
        log::debug!("Gates IR: {gates:?}");

        log::debug!(
            "Lowering inter region equality constraints for group {:?}",
            group.name()
        );
        let eq_constraints = select_equality_constraints(group, ctx, &free_cells.inputs);
        log::debug!(
            "[{}] Equality constraints: {:?}",
            group.name(),
            eq_constraints
        );
        let eq_constraints = IRStmt::seq(inter_region_constraints(
            eq_constraints,
            &advice_io,
            &instance_io,
            ctx.syn().fixed_query_resolver(),
        ));
        // Relativize the advice cells used in the constraints
        // TODO: Move this to the resolve method.

        log::debug!(
            "[{}] Equality constraints (lowered): {eq_constraints:?}",
            group.name()
        );

        log::debug!("Lowering lookups for group {:?}", group.name());
        let lookups = IRStmt::seq(codegen_lookup_invocations(
            ctx.syn(),
            &group.region_rows(advice_io, instance_io, ctx.syn().fixed_query_resolver()),
            ctx.lookup_cb(),
        )
        //.and_then(scoped_exprs_to_aexpr)
        ?);

        //        log::debug!("Adding injected IR for group {:?}", group.name());
        //        let mut injected = vec![];
        //for region in group.regions() {
        //                    let index = region
        //                        .index()
        //                        .ok_or_else(|| anyhow::anyhow!("Region does not have an index"))?;
        //                    let start = region
        //                        .start().unwrap_or_default();
        //                    if let Some(ir) = injector.inject(index, start) {
        //                        injected.push(crate::backend::codegen::lower_injected_ir(
        //                            ir,
        //                            region,
        //                            &advice_io,
        //                            &instance_io,
        //                            ctx.syn().fixed_query_resolver(),
        //                        )?);
        //                    }
        //                }

        Ok(Self {
            id,
            input_count: instance_io.inputs().len() + advice_io.inputs().len(),
            output_count: instance_io.outputs().len() + advice_io.outputs().len(),
            name: group.name().to_owned(),
            key: group.key(),
            callsites,
            gates,
            eq_constraints,
            lookups,
            injected: vec![],
        })
    }

    /// Injects IR into the group scoped by the region.
    pub(super) fn inject_ir<'a>(
        &'a mut self,
        region: RegionData<'s>,
        ir: &IRStmt<ExpressionInRow<'s, F>>,
        advice_io: &'s CircuitIO<Advice>,
        instance_io: &'s CircuitIO<Instance>,
        fqr: &'s dyn FixedQueryResolver<F>,
    ) {
        self.injected.push(
            ir.map_into(&|expr| expr.scoped_in_region_row(region, advice_io, instance_io, fqr)),
        )
    }
}

impl GroupBody<IRAexpr> {
    /// Relativizes advice cells to the regions in the group.
    ///
    /// It is used for improving the detection of equivalent groups.
    pub fn relativize_eq_constraints(&mut self, ctx: &IRCtx) -> anyhow::Result<()> {
        self.eq_constraints.try_map_inplace(&|expr| {
            expr.try_map_io(&|io| match io {
                FuncIO::Advice(cell) => {
                    *cell = try_relativize_advice_cell(
                        *cell,
                        ctx.regions_by_index().values().copied(),
                    )?;
                    Ok(())
                }
                _ => Ok(()),
            })
        })
    }
}

impl<E> GroupBody<E> {
    pub fn is_main(&self) -> bool {
        self.key.is_none()
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    pub fn callsites(&self) -> &[CallSite<E>] {
        &self.callsites
    }

    pub fn callsites_mut(&mut self) -> &mut Vec<CallSite<E>> {
        &mut self.callsites
    }
    pub fn key(&self) -> Option<GroupKeyInstance> {
        self.key
    }

    pub fn try_map<O>(self, f: &impl Fn(E) -> Result<O>) -> Result<GroupBody<O>> {
        Ok(GroupBody {
            name: self.name,
            id: self.id,
            input_count: self.input_count,
            output_count: self.output_count,
            key: self.key,
            gates: self.gates.try_map(f)?,
            eq_constraints: self.eq_constraints.try_map(f)?,
            callsites: self
                .callsites
                .into_iter()
                .map(|cs| cs.try_map(f))
                .collect::<Result<Vec<_>, _>>()?,
            lookups: self.lookups.try_map(f)?,
            injected: self
                .injected
                .into_iter()
                .map(|i| i.try_map(f))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl EqvRelation<GroupBody<IRAexpr>> for SymbolicEqv {
    /// Two groups are equivalent if the code they represent is equivalent and have the same key.
    ///
    /// Special case is main which is never equivalent to anything.
    fn equivalent(lhs: &GroupBody<IRAexpr>, rhs: &GroupBody<IRAexpr>) -> bool {
        // Main is never equivalent to others
        if lhs.is_main() || rhs.is_main() {
            return false;
        }

        let lhs_key = lhs.key.unwrap();
        let rhs_key = rhs.key.unwrap();

        let k = lhs_key == rhs_key;
        log::debug!("[equivalent({} ~ {})] key: {k}", lhs.id(), rhs.id());
        let io = lhs.input_count == rhs.input_count && lhs.output_count == rhs.output_count;
        log::debug!("[equivalent({} ~ {})] io: {io}", lhs.id(), rhs.id());
        let gates = Self::equivalent(&lhs.gates, &rhs.gates);
        log::debug!("[equivalent({} ~ {})] gates: {gates}", lhs.id(), rhs.id());
        let eqc = Self::equivalent(&lhs.eq_constraints, &rhs.eq_constraints);
        log::debug!("[equivalent({} ~ {})] eqc: {eqc}", lhs.id(), rhs.id());
        let lookups = Self::equivalent(&lhs.lookups, &rhs.lookups);
        log::debug!(
            "[equivalent({} ~ {})] lookups: {lookups}",
            lhs.id(),
            rhs.id()
        );
        let callsites = Self::equivalent(&lhs.callsites, &rhs.callsites);
        log::debug!(
            "[equivalent({} ~ {})] callsites: {callsites}",
            lhs.id(),
            rhs.id()
        );

        k && io && gates && eqc && lookups && callsites
    }
}

impl LowerableStmt for GroupBody<IRAexpr> {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        l.generate_comment("Calls to subgroups".to_owned())?;
        for callsite in self.callsites {
            callsite.lower(l)?;
        }
        l.generate_comment("Gate constraints".to_owned())?;
        self.gates.lower(l)?;
        l.generate_comment("Equality constraints".to_owned())?;
        self.eq_constraints.lower(l)?;
        l.generate_comment("Lookups".to_owned())?;
        self.lookups.lower(l)?;
        l.generate_comment("Injected".to_owned())?;
        for stmt in self.injected {
            stmt.lower(l)?;
        }

        Ok(())
    }
}

impl<E: Clone> Clone for GroupBody<E> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            id: self.id.clone(),
            input_count: self.input_count,
            output_count: self.output_count,
            key: self.key.clone(),
            gates: self.gates.clone(),
            eq_constraints: self.eq_constraints.clone(),
            callsites: self.callsites.clone(),
            lookups: self.lookups.clone(),
            injected: self.injected.clone(),
        }
    }
}

/// Select the equality constraints that concern this group.
pub fn select_equality_constraints<F: Field>(
    group: &Group,
    ctx: &GroupIRCtx<'_, '_, F>,
    free_inputs: &[GroupCell],
) -> Vec<EqConstraint<F>> {
    let bounds = GroupBounds::new_with_extra(group, ctx.regions_by_index(), Some(free_inputs));

    ctx.syn()
        .constraints()
        .edges()
        .into_iter()
        .filter(|c| {
            log::debug!("Checking if eq constraint should go: {c:?}");
            match bounds.check_eq_constraint(c) {
                EqConstraintCheck::AnyToAny(left, l, right, r) => match (left, right) {
                    (Bound::Within, Bound::Within) => true,
                    (Bound::Within, Bound::ForeignIO) => true,
                    (Bound::ForeignIO, Bound::Within) => true,
                    (Bound::Within, Bound::IO) => true,
                    (Bound::IO, Bound::Within) => true,
                    (Bound::IO, Bound::IO) => true,
                    (Bound::IO, Bound::ForeignIO) => true,
                    (Bound::ForeignIO, Bound::IO) => true,
                    (Bound::ForeignIO, Bound::ForeignIO) => false,
                    (Bound::ForeignIO, Bound::Outside) => false,
                    (Bound::Outside, Bound::ForeignIO) => false,
                    (Bound::Outside, Bound::Outside) => false,
                    (Bound::IO, Bound::Outside) => false,
                    (Bound::Outside, Bound::IO) => false,
                    (Bound::Within, Bound::Outside) => match r.0.column_type() {
                        crate::halo2::Any::Fixed => true,
                        _ => unreachable!("Within {l:?} | Outside {r:?}"),
                    },

                    (Bound::Outside, Bound::Within) => match l.0.column_type() {
                        crate::halo2::Any::Fixed => true,
                        _ => unreachable!("Outside {l:?} | Within {r:?}"),
                    },
                },
                EqConstraintCheck::FixedToConst(bound) => match bound {
                    Bound::Within | Bound::Outside => true,
                    _ => unreachable!(),
                },
            }
        })
        .collect()
}

/// Generates constraint expressions for the equality constraints.
///
/// This function accepts an iterator of equality constraints to facilitate
/// filtering the equality constraints of a group from the global equality constraints graph.
pub fn inter_region_constraints<'s, F: Field>(
    constraints: impl IntoIterator<Item = EqConstraint<F>>,
    advice_io: &'s CircuitIO<Advice>,
    instance_io: &'s CircuitIO<Instance>,
    fixed_query_resolver: &'s dyn FixedQueryResolver<F>,
) -> Vec<IRStmt<ScopedExpression<'s, 's, F>>> {
    constraints
        .into_iter()
        .map(|constraint| match constraint {
            EqConstraint::AnyToAny(from, from_row, to, to_row) => (
                ScopedExpression::new(
                    from.query_cell(Rotation::cur()),
                    Row::new(from_row, advice_io, instance_io, fixed_query_resolver),
                ),
                ScopedExpression::new(
                    to.query_cell(Rotation::cur()),
                    Row::new(to_row, advice_io, instance_io, fixed_query_resolver),
                ),
            ),
            EqConstraint::FixedToConst(column, row, f) => (
                ScopedExpression::new(
                    column.query_cell(Rotation::cur()),
                    Row::new(row, advice_io, instance_io, fixed_query_resolver),
                ),
                ScopedExpression::new(
                    Expression::Constant(f),
                    Row::new(row, advice_io, instance_io, fixed_query_resolver),
                ),
            ),
        })
        .map(|(lhs, rhs)| IRStmt::constraint(CmpOp::Eq, lhs, rhs))
        .collect()
}

/// Uses the given rewrite patterns to lower the gates on each region.
fn lower_gates<'a, 'r, F: Field>(
    gates: &'a [Gate<F>],
    regions: &'r [RegionData<'a>],
    patterns: &RewritePatternSet<F>,
    advice_io: &'a CircuitIO<Advice>,
    instance_io: &'a CircuitIO<Instance>,
    fqr: &'a dyn FixedQueryResolver<F>,
) -> Result<Vec<IRStmt<ScopedExpression<'a, 'a, F>>>> {
    log::debug!("Got {} gates and {} regions", gates.len(), regions.len());
    utils::product(regions, gates)
        .map(|(r, g)| {
            log::debug!("Lowering gate {} in region {}", g.name(), r.name());
            let rows = r.rows();
            let scope = GateScope::new(g, *r, (rows.start, rows.end), advice_io, instance_io, fqr);

            patterns
                .match_and_rewrite(scope)
                .map_err(|e| make_error(e, scope))
                .and_then(|stmt| {
                    stmt.try_map(&|(row, expr)| {
                        let rr = scope.region_row(row)?;
                        Ok(ScopedExpression::from_cow(expr, rr))
                    })
                })
                .map(|stmt| {
                    prepend_comment(
                        stmt,
                        IRStmt::comment(format!(
                            "gate '{}' @ {} @ rows {}..={}",
                            scope.gate_name(),
                            scope.region_header().to_string(),
                            scope.start_row(),
                            scope.end_row()
                        )),
                    )
                })
        })
        .collect()
}

/// If the given statement is not empty prepends a comment
/// with contextual information.
#[inline]
fn prepend_comment<'a, F: Field>(
    stmt: IRStmt<ScopedExpression<'a, 'a, F>>,
    comment: IRStmt<ScopedExpression<'a, 'a, F>>,
) -> IRStmt<ScopedExpression<'a, 'a, F>> {
    if stmt.is_empty() {
        return stmt;
    }
    [comment, stmt].into_iter().collect()
}

/// If the rewrite error is [`RewriteError::NoMatch`] returns an error
/// that the gate in scope did not match any pattern. If it is [`RewriteError::Err`]
/// forwards the inner error.
#[inline]
fn make_error<F>(e: RewriteError, scope: GateScope<F>) -> anyhow::Error
where
    F: Field,
{
    match e {
        RewriteError::NoMatch => anyhow::anyhow!(
            "Gate '{}' on region {} '{}' did not match any pattern",
            scope.gate_name(),
            scope
                .region_index()
                .as_deref()
                .map(ToString::to_string)
                .unwrap_or("unk".to_string()),
            scope.region_name()
        ),
        RewriteError::Err(error) => anyhow::anyhow!(error),
    }
}
