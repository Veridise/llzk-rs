use crate::{
    backend::{
        codegen::{
            inter_region_constraints,
            lookup::codegen_lookup_invocations,
            scoped_exprs_to_aexpr,
            strats::{
                groups::{
                    bounds::{Bound, EqConstraintCheck, GroupBounds},
                    callsite::CallSite,
                    free_cells::FreeCells,
                    GroupIRCtx,
                },
                lower_gates,
            },
        }, func::{try_relativize_advice_cell, FuncIO}, lowering::{lowerable::LowerableStmt, Lowering}
    },
    halo2::{groups::GroupKeyInstance, Advice, Field, Instance},
    ir::{
        equivalency::{EqvRelation, SymbolicEqv},
        expr::IRAexpr,
        stmt::IRStmt,
    },
    synthesis::{
        constraint::EqConstraint,
        groups::{Group, GroupCell},
    },
    CircuitIO,
};
use anyhow::Result;
use std::{borrow::Cow, collections::HashMap};

/// Organizes the groups by their key. Each group contains a list with the index to the group.
pub fn organize_groups_by_key<F: Field>(
    groups: &[GroupBody<F>],
) -> HashMap<GroupKeyInstance, Vec<usize>> {
    let mut groups_by_key: HashMap<_, Vec<_>> = HashMap::new();
    for group in groups {
        if group.is_main() {
            log::debug!("Group {} is main. Skipping...", group.id);
            continue;
        }
        groups_by_key
            .entry(group.key.expect("Non main group needs a key"))
            .or_default()
            .push(group.id);
        log::debug!("Inserting group {} with key {:?}", group.id, group.key);
    }
    groups_by_key
}

/// Group's IR
#[derive(Debug)]
pub struct GroupBody<F> {
    name: String,
    /// Index in the original groups array.
    id: usize,
    io: (usize, usize),
    key: Option<GroupKeyInstance>,
    gates: IRStmt<IRAexpr<F>>,
    eq_constraints: IRStmt<IRAexpr<F>>,
    callsites: Vec<CallSite<F>>,
    lookups: IRStmt<IRAexpr<F>>,
    injected: Vec<IRStmt<IRAexpr<F>>>
}

/// If the group has free cells that need to be bounded and is not the top level group
/// makes a copy of its IO and adds the cells as inputs.
fn updated_io<'a>(
    group: &'a Group,
    free_cells: &FreeCells,
) -> (Cow<'a, CircuitIO<Advice>>, Cow<'a, CircuitIO<Instance>>) {
    // Use a Cow to avoid cloning unless we have to.
    let mut advice_io = Cow::Borrowed(group.advice_io());
    let mut instance_io = Cow::Borrowed(group.instance_io());

    // Do not update the IO if it's main.
    if group.is_top_level() {
        return (advice_io, instance_io);
    }
    for cell in &free_cells.inputs {
        match cell {
            GroupCell::InstanceIO(cell) => instance_io.to_mut().add_input(*cell),
            GroupCell::AdviceIO(cell) => advice_io.to_mut().add_input(*cell),
            GroupCell::Assigned(_) => unreachable!(),
        }
    }

    (advice_io, instance_io)
}



impl<F: Field> GroupBody<F> {
    pub(super) fn new(
        group: &Group,
        id: usize,
        ctx: GroupIRCtx<'_, '_, F>,
        free_cells: &FreeCells,
    injector: & mut dyn crate::IRInjectCallback<F>,
    ) -> anyhow::Result<Self> {
        let (advice_io, instance_io) = updated_io(group, free_cells);
               log::debug!("Lowering call-sites for group {:?}", group.name());
        let callsites = {
            group
                .children(ctx.groups)
                .into_iter()
                .enumerate()
                .map(|(call_no, (callee_id, callee))| {
                    CallSite::new(
                        callee,
                        callee_id,
                        ctx,
                        call_no,
                        &advice_io,
                        &instance_io,
                        &free_cells.callsites[call_no],
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        log::debug!("Lowering gates for group {:?}", group.name());
        let gates = lower_gates(
            ctx.syn.gates(),
            &group.regions(),
            ctx.patterns,
            &advice_io,
            &instance_io,
            ctx.syn.fixed_query_resolver(),
        )
        .and_then(scoped_exprs_to_aexpr)?;
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
        let eq_constraints = scoped_exprs_to_aexpr(inter_region_constraints(
            eq_constraints,
            &advice_io,
            &instance_io,
            ctx.syn.fixed_query_resolver(),
        )).and_then(|eq_constraints| 
        // Relativize the advice cells used in the constraints
        eq_constraints.try_map(&|expr| expr.try_map_io(&|io| Ok(match io {
                    FuncIO::Advice(cell) => FuncIO::Advice(try_relativize_advice_cell(cell, ctx.regions_by_index.values().copied())?),
                    f => f
                }))))?;

        log::debug!(
            "[{}] Equality constraints (lowered): {eq_constraints:?}",
            group.name()
        );

        log::debug!("Lowering lookups for group {:?}", group.name());
        let lookups = codegen_lookup_invocations(
            ctx.syn,
            group.region_rows(ctx.syn.fixed_query_resolver()).as_slice(),
            ctx.lookup_cb,
        )
        .and_then(scoped_exprs_to_aexpr)?;

        log::debug!("Adding injected IR for group {:?}", group.name());
        let mut injected = vec![];
for region in group.regions() {
                    let index = region
                        .index()
                        .ok_or_else(|| anyhow::anyhow!("Region does not have an index"))?;
                    let start = region
                        .start()
                        .ok_or_else(|| anyhow::anyhow!("Region does not have a start row"))?;
                    if let Some(ir) = injector.inject(index, start) {
                        injected.push(crate::backend::codegen::lower_injected_ir(
                            ir,
                            region,
                            &advice_io,
                            &instance_io,
                            ctx.syn.fixed_query_resolver(),
                        )?);
                    }
                }

        let n_inputs = instance_io.inputs().len() + advice_io.inputs().len();
        let n_outputs = instance_io.outputs().len() + advice_io.outputs().len();

        Ok(Self {
            id,
            io: (n_inputs, n_outputs),
            name: group.name().to_owned(),
            key: group.key(),
            callsites,
            gates,
            eq_constraints,
            lookups,
            injected
        })
    }

    pub fn is_main(&self) -> bool {
        self.key.is_none()
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn io(&self) -> (usize, usize) {
        self.io
    }

    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    pub fn callsites(&self) -> &[CallSite<F>] {
        &self.callsites
    }

    pub fn callsites_mut(&mut self) -> &mut Vec<CallSite<F>> {
        &mut self.callsites
    }


}

impl<F: Field> EqvRelation<GroupBody<F>> for SymbolicEqv {
    /// Two groups are equivalent if the code they represent is equivalent and have the same key.
    ///
    /// Special case is main which is never equivalent to anything.
    fn equivalent(lhs: &GroupBody<F>, rhs: &GroupBody<F>) -> bool {
        // Main is never equivalent to others
        if lhs.is_main() || rhs.is_main() {
            return false;
        }

        let lhs_key = lhs.key.unwrap();
        let rhs_key = rhs.key.unwrap();

        let k = lhs_key == rhs_key;
        let io = lhs.io == rhs.io;
        let gates = Self::equivalent(&lhs.gates, &rhs.gates);
        let eqc = Self::equivalent(&lhs.eq_constraints, &rhs.eq_constraints);
        let lookups = Self::equivalent(&lhs.lookups, &rhs.lookups);
        let callsites = Self::equivalent(&lhs.callsites, &rhs.callsites);

        k && io && gates && eqc && lookups && callsites
    }
}

impl<F: Field> LowerableStmt for GroupBody<F> {
    type F = F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
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

/// Select the equality constraints that concern this group.
pub fn select_equality_constraints<F: Field>(
    group: &Group,
    ctx: super::GroupIRCtx<'_, '_, F>,
    free_inputs: &[GroupCell],
) -> Vec<EqConstraint<F>> {
    let bounds = GroupBounds::new_with_extra(group, ctx, Some(free_inputs));

    ctx.syn
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
                    } 

                    
                    (Bound::Outside, Bound::Within) => match l.0.column_type() {
                        crate::halo2::Any::Fixed => true,
                        _ => unreachable!("Outside {l:?} | Within {r:?}"),
                    } 
  
                    
                },
                EqConstraintCheck::FixedToConst(bound) => match bound {
                    Bound::Within | 
                    Bound::Outside => true,
                    _ => unreachable!(),
                },
            }
        })
        .collect()
}
