//! Structs for handling the IR of groups of regions inside the circuit.

use crate::{
    backend::{
        func::{CellRef, FuncIO},
        lowering::{lowerable::LowerableStmt, Lowering},
    },
    expressions::{ExpressionInRow, ScopedExpression},
    gates::RewritePatternSet,
    halo2::{groups::GroupKeyInstance, Expression, Field, Gate, Rotation},
    ir::{
        ctx::AdviceCells,
        equivalency::{EqvRelation, SymbolicEqv},
        expr::{Felt, IRAexpr},
        generate::{free_cells::FreeCells, lookup::codegen_lookup_invocations, GroupIRCtx},
        groups::{
            bounds::{Bound, EqConstraintCheck, GroupBounds},
            callsite::CallSite,
        },
        stmt::IRStmt,
        CmpOp, IRCtx,
    },
    resolvers::FixedQueryResolver,
    synthesis::{
        constraint::EqConstraint,
        groups::{Group, GroupCell},
        regions::{RegionData, Row},
    },
    utils, GateRewritePattern as _, GateScope, RewriteError,
};
use anyhow::Result;

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

impl<'cb, 'syn, 'ctx, 'sco, F> GroupBody<ScopedExpression<'syn, 'sco, F>>
where
    'cb: 'sco + 'syn,
    'syn: 'sco,
    'ctx: 'sco + 'syn,
    F: Field,
{
    pub(super) fn new(
        group: &'syn Group,
        id: usize,
        ctx: &GroupIRCtx<'cb, 'syn, F>,
        free_cells: &'ctx FreeCells,
        advice_io: &'ctx crate::io::AdviceIO,
        instance_io: &'ctx crate::io::InstanceIO,
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
            advice_io,
            instance_io,
            ctx.syn().fixed_query_resolver(),
        )?);
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
            advice_io,
            instance_io,
            ctx.syn().fixed_query_resolver(),
        ));

        log::debug!(
            "[{}] Equality constraints (lowered): {eq_constraints:?}",
            group.name()
        );

        log::debug!("Lowering lookups for group {:?}", group.name());
        let lookups = IRStmt::seq(codegen_lookup_invocations(
            ctx.syn(),
            &group.region_rows(advice_io, instance_io, ctx.syn().fixed_query_resolver()),
            ctx.lookup_cb(),
        )?);

        let mut injected = vec![IRStmt::comment(format!(
            "free cells: {:?}",
            free_cells.inputs
        ))];
        for (n, callsite) in free_cells.callsites.iter().enumerate() {
            injected.push(IRStmt::comment(format!(
                " call {n}  free cells: {callsite:?}"
            )));
        }
        for (n, (col, row)) in instance_io.inputs().iter().enumerate() {
            injected.push(IRStmt::comment(format!(
                " in{n} (instance): {}, {row}",
                col.index()
            )));
        }
        let l = instance_io.inputs().len();
        for (n, (col, row)) in advice_io.inputs().iter().enumerate() {
            injected.push(IRStmt::comment(format!(
                " in{} (advice): {}, {row}",
                n + l,
                col.index()
            )));
        }
        for (n, (col, row)) in instance_io.outputs().iter().enumerate() {
            injected.push(IRStmt::comment(format!(
                " out{n} (instance): {}, {row}",
                col.index()
            )));
        }
        let l = instance_io.outputs().len();
        for (n, (col, row)) in advice_io.outputs().iter().enumerate() {
            injected.push(IRStmt::comment(format!(
                " out{} (advice): {}, {row}",
                n + l,
                col.index()
            )));
        }
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
            injected,
        })
    }

    /// Injects IR into the group scoped by the region.
    pub(super) fn inject_ir<'a>(
        &'a mut self,
        region: RegionData<'syn>,
        ir: &IRStmt<ExpressionInRow<'syn, F>>,
        advice_io: &'ctx crate::io::AdviceIO,
        instance_io: &'ctx crate::io::InstanceIO,
        fqr: &'syn dyn FixedQueryResolver<F>,
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
                    *cell = try_relativize_advice_cell(*cell, ctx.advice_cells().values())?;
                    Ok(())
                }
                _ => Ok(()),
            })
        })
    }

    /// Folds the statements if the expressions are constant.
    ///
    /// If any of the statements fails to fold returns an error.
    pub(crate) fn constant_fold(&mut self, prime: Felt) -> Result<()> {
        self.gates.constant_fold(prime)?;
        self.eq_constraints.constant_fold(prime)?;
        for callsite in &mut self.callsites {
            callsite.constant_fold(prime);
        }
        self.lookups.constant_fold(prime)?;
        self.injected
            .iter_mut()
            .try_for_each(|s| s.constant_fold(prime))
    }

    /// Matches the statements against a series of known patterns and applies rewrites if able to.
    pub fn canonicalize(&mut self) {
        self.gates.canonicalize();
        self.eq_constraints.canonicalize();
        self.lookups.canonicalize();
        for stmt in &mut self.injected {
            stmt.canonicalize();
        }
    }
}

/// Searches to what region the advice cell belongs to and converts it to a relative reference from
/// that region.
///
/// Fails if the advice cell could not be found in any region.
fn try_relativize_advice_cell<'a>(
    cell: CellRef,
    regions: impl IntoIterator<Item = &'a AdviceCells>,
) -> anyhow::Result<CellRef> {
    if !cell.is_absolute() {
        return Ok(cell);
    }
    for region in regions {
        if !region.contains_advice_cell(cell.col(), cell.row()) {
            continue;
        }
        let start = region
            .start()
            .ok_or_else(|| anyhow::anyhow!("Region does not have a base"))?;
        return cell
            .relativize(start)
            .ok_or_else(|| anyhow::anyhow!("Failed to relativize cell"));
    }

    Err(anyhow::anyhow!(
        "cell reference {cell:?} was not found in any region"
    ))
}

impl<E> GroupBody<E> {
    /// Returns true if the group is the top-level.
    pub fn is_main(&self) -> bool {
        self.key.is_none()
    }

    /// Returns the index in the groups list.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Returns the name of the group.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a mutable reference to the name.
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    /// Returns the list of callsites inside the group.
    pub fn callsites(&self) -> &[CallSite<E>] {
        &self.callsites
    }

    /// Returns a mutable reference to the callsites list inside the group.
    pub fn callsites_mut(&mut self) -> &mut Vec<CallSite<E>> {
        &mut self.callsites
    }

    /// Returns the group key. Returns `None` if the group is the top-level.
    pub fn key(&self) -> Option<GroupKeyInstance> {
        self.key
    }

    /// Tries to convert the inner expression type to another.
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
            id: self.id,
            input_count: self.input_count,
            output_count: self.output_count,
            key: self.key,
            gates: self.gates.clone(),
            eq_constraints: self.eq_constraints.clone(),
            callsites: self.callsites.clone(),
            lookups: self.lookups.clone(),
            injected: self.injected.clone(),
        }
    }
}

/// Select the equality constraints that concern this group.
fn select_equality_constraints<F: Field>(
    group: &Group,
    ctx: &GroupIRCtx<'_, '_, F>,
    free_inputs: &[GroupCell],
) -> Vec<EqConstraint<F>> {
    let bounds = GroupBounds::new_with_extra(
        group,
        ctx.groups(),
        ctx.regions_by_index(),
        Some(free_inputs),
    );

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
fn inter_region_constraints<'e, 's, F: Field>(
    constraints: impl IntoIterator<Item = EqConstraint<F>>,
    advice_io: &'s crate::io::AdviceIO,
    instance_io: &'s crate::io::InstanceIO,
    fixed_query_resolver: &'s dyn FixedQueryResolver<F>,
) -> Vec<IRStmt<ScopedExpression<'e, 's, F>>> {
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
fn lower_gates<'sco, 'syn, 'io, F>(
    gates: &'syn [Gate<F>],
    regions: &[RegionData<'syn>],
    patterns: &RewritePatternSet<F>,
    advice_io: &'io crate::io::AdviceIO,
    instance_io: &'io crate::io::InstanceIO,
    fqr: &'syn dyn FixedQueryResolver<F>,
) -> Result<Vec<IRStmt<ScopedExpression<'syn, 'sco, F>>>>
where
    'syn: 'sco,
    'io: 'sco + 'syn,
    F: Field,
{
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
