use crate::{
    backend::{
        codegen::{
            inter_region_constraints,
            lookup::{codegen_lookup_invocations, codegen_lookup_modules},
            lower_constraints, scoped_exprs_to_aexpr,
            strats::{load_patterns, lower_gates},
            Codegen, CodegenStrategy,
        },
        func::FuncIO,
        lowering::{
            lowerable::{LowerableExpr as _, LowerableStmt},
            Lowering,
        },
        resolvers::ResolversProvider,
    },
    expressions::{utils::ExprDebug, ScopedExpression},
    gates::{
        find_selectors, GateRewritePattern, GateScope, RewriteError, RewriteOutput,
        RewritePatternSet,
    },
    halo2::{groups::GroupKeyInstance, Expression, Field, RegionIndex},
    ir::{
        equivalency::{EqvRelation, SymbolicEqv},
        expr::IRAexpr,
        stmt::{chain_lowerable_stmts, IRStmt},
        CmpOp,
    },
    lookups::callbacks::LookupCallbacks,
    synthesis::{
        constraint::EqConstraint,
        groups::{Group, GroupCell},
        regions::{RegionData, RegionRow, RegionRowLike as _, Row},
        CircuitSynthesis,
    },
    utils, CircuitIO, GateCallbacks,
};
use anyhow::Result;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    result::Result as StdResult,
};

fn header_comments<F: Field, S: ToString>(s: S) -> Vec<IRStmt<(F,)>> {
    s.to_string().lines().map(IRStmt::comment).collect()
}

/// Code generation strategy that write the code of each group in a separate function.
#[derive(Default)]
pub struct GroupConstraintsStrat {}

impl CodegenStrategy for GroupConstraintsStrat {
    fn codegen<'c: 'st, 's, 'st, C>(
        &self,
        codegen: &C,
        syn: &'s CircuitSynthesis<C::F>,
        lookup_cb: &dyn LookupCallbacks<C::F>,
        gate_cbs: &dyn GateCallbacks<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c, 'st>,
        Row<'s>: ResolversProvider<C::F> + 's,
        RegionRow<'s, 's>: ResolversProvider<C::F> + 's,
    {
        let patterns = load_patterns(gate_cbs);
        let regions = syn
            .groups()
            .iter()
            .flat_map(|g| g.regions())
            .map(|r| {
                r.index()
                    .map(|i| (i, r))
                    .ok_or_else(|| anyhow::anyhow!("Region {r:?} does not have an index"))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        let ctx = GroupIRCtx {
            groups: syn.groups(),
            regions_by_index: &regions,
            syn,
            patterns: &patterns,
            lookup_cb,
        };
        assert_eq!(ctx.regions_by_index.len(), ctx.groups.len());

        log::debug!("Generating IR of region groups");
        let mut groups_ir = ctx
            .groups
            .iter()
            .enumerate()
            .map(|(id, g)| GroupBody::new(g, id, ctx))
            .collect::<Result<Vec<_>, _>>()?;

        // Sanity check, only one group should be considered main.
        assert_eq!(
            groups_ir.iter().filter(|g| g.is_main()).count(),
            1,
            "Only one main group is allowed"
        );
        // Select leaders and generate the final names.
        // If the group was renamed its index with contain Some(_).
        let (leaders, updated_calldata) = select_leaders(&groups_ir);

        // Build the final list of IR and invoke codegen
        groups_ir.retain_mut(|g| {
            // Keep a group if its main or is in the leaders list.
            let keep = g.is_main() || leaders.contains(&g.id);
            if keep {
                // If we are keeping it update the names if necessary.
                update_names(g, &updated_calldata)
            }
            keep
        });

        // Create a function per group.
        for group in groups_ir {
            if group.is_main() {
                log::debug!("Generating main body");
                codegen.within_main(syn, move |_| Ok([group]))?;
            } else {
                log::debug!("Generating body of function {}", group.name);
                codegen.define_function_with_body(
                    &group.name.clone(),
                    group.io.0,
                    group.io.1,
                    |_, _, _| Ok([group]),
                )?;
            }
        }
        Ok(())
    }
}

/// Find the leaders of each equivalence class in the groups and annotate the required renames
fn select_leaders<F: Field>(groups_ir: &[GroupBody<F>]) -> (Vec<usize>, Vec<Option<String>>) {
    // Separate the groups by their key and since main does not have a key we keep it separate.
    let groups_by_key = organize_groups_by_key(&groups_ir);
    let mut leaders = vec![];
    // For each group annotate its new name if it needs to be renamed.
    let mut updated_calldata: Vec<Option<String>> = vec![None; groups_ir.len()];
    // Avoids duplicating names
    let mut used_names: HashSet<String> = HashSet::default();
    // For each set of groups with the same key we create equivalence classes and select a
    // leader for each class.
    for (_, groups) in &groups_by_key {
        let mut eqv_class = disjoint::DisjointSet::new();
        // Find the equivalence classes.
        for (i, j) in utils::product(groups.as_slice(), groups.as_slice()) {
            if i == j {
                continue;
            }
            let lhs = &groups_ir[*i];
            let rhs = &groups_ir[*j];
            if SymbolicEqv::equivalent(lhs, rhs) {
                eqv_class.join(*i, *j);
            }
        }
        // For each group eqv class select a leader and annotate what groups need to be updated.
        for (n, set) in eqv_class.sets().into_iter().enumerate() {
            debug_assert!(!set.is_empty());
            // We arbitrarily chose the leader to be the first element.
            leaders.push(set[0]);
            let leader = groups_ir.get(set[0]).unwrap();
            let name = fresh_group_name(&leader.name, &mut used_names, n);
            for update in &set {
                updated_calldata[*update] = Some(name.clone());
            }
        }
    }

    (leaders, updated_calldata)
}

/// Updates the name of the group and the names of the functions each callsite references.
fn update_names<F: Field>(group: &mut GroupBody<F>, updated_calldata: &[Option<String>]) {
    if let Some(name) = &updated_calldata[group.id] {
        group.name = name.clone();
    }
    for callsite in &mut group.callsites {
        if let Some(name) = &updated_calldata[callsite.callee_id] {
            callsite.name = name.clone();
        }
    }
}

/// Finds a version of the given name that is fresh.
fn fresh_group_name(name: &str, used_names: &mut HashSet<String>, n: usize) -> String {
    // Create a lazy iterator with the input name and every rename and then consume it until we get
    // a valid name.
    let name = std::iter::chain([name.to_owned()], (n..).map(|n| format!("{name}{n}")))
        .find_map(|name| {
            if used_names.contains(&name) {
                return None;
            }
            Some(name)
        })
        .unwrap();
    used_names.insert(name.clone());
    name
}

/// Organizes the groups by their key. Each group contains a list with the index to the group.
fn organize_groups_by_key<F: Field>(
    groups: &[GroupBody<F>],
) -> HashMap<GroupKeyInstance, Vec<usize>> {
    let mut groups_by_key: HashMap<_, Vec<_>> = HashMap::new();
    for group in groups {
        if group.is_main() {
            continue;
        }
        groups_by_key
            .entry(group.key.expect("Non main group needs a key"))
            .or_default()
            .push(group.id);
    }
    groups_by_key
}

/// Support data for creating group body IR structs
#[derive(Copy, Clone)]
struct GroupIRCtx<'g, 's, F: Field> {
    groups: &'g [Group],
    regions_by_index: &'g HashMap<RegionIndex, RegionData<'s>>,
    syn: &'s CircuitSynthesis<F>,
    patterns: &'g RewritePatternSet<F>,
    lookup_cb: &'g dyn LookupCallbacks<F>,
}

/// Data related to a single callsite
struct CallSite<F> {
    name: String,
    callee: GroupKeyInstance,
    /// The index in the original groups array to the called group.
    callee_id: usize,
    call_no: usize,
    inputs: Vec<IRAexpr<F>>,
    output_vars: Vec<FuncIO>,
    outputs: Vec<IRAexpr<F>>,
}

fn cells_to_exprs<F: Field>(
    cells: &[GroupCell],
    ctx: GroupIRCtx<'_, '_, F>,
    advice_io: &CircuitIO<crate::halo2::Advice>,
    instance_io: &CircuitIO<crate::halo2::Instance>,
) -> anyhow::Result<Vec<IRAexpr<F>>> {
    cells
        .into_iter()
        .map(|cell| {
            let region: Option<RegionData<'_>> = cell
                .region_index()
                .map(|index| {
                    ctx.regions_by_index.get(&index).ok_or_else(|| {
                        anyhow::anyhow!("Region with index {} is not a known region", *index)
                    })
                })
                .transpose()?
                .copied();

            let expr = cell.to_expr::<F>();
            let row = cell.row();
            match region {
                Some(region) => {
                    ScopedExpression::new(expr, RegionRow::new(region, row, advice_io, instance_io))
                }
                None => ScopedExpression::new(expr, Row::new(row, advice_io, instance_io)),
            }
            .try_into()
        })
        .collect()
}

impl<F: Field> EqvRelation<CallSite<F>> for SymbolicEqv {
    /// Two callsites are equivalent if the call statement they represent is equivalent.
    fn equivalent(lhs: &CallSite<F>, rhs: &CallSite<F>) -> bool {
        lhs.callee == rhs.callee
            && Self::equivalent(&lhs.inputs, &rhs.inputs)
            && Self::equivalent(&lhs.outputs, &rhs.outputs)
    }
}

impl<F: Field> CallSite<F> {
    pub fn new(
        caller: &Group,
        callee: &Group,
        callee_id: usize,
        ctx: GroupIRCtx<'_, '_, F>,
        call_no: usize,
    ) -> anyhow::Result<Self> {
        let advice_io = caller.advice_io();
        let instance_io = caller.instance_io();
        let callee_key = callee
            .key()
            .ok_or_else(|| anyhow::anyhow!("Top level cannot be called by other group"))?;

        let inputs = cells_to_exprs(callee.inputs(), ctx, advice_io, instance_io)?;
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
            call_no,
        })
    }

    /// Returns true if the callsite are structurally equivalent and point to the same group key.
    pub fn equivalent(&self, other: &Self) -> bool {
        // Group key, callee id, and input/output arity must be equal.
        if self.callee != other.callee || self.callee_id != other.callee_id {
            return false;
        }

        true
    }
}

impl<F: Field> LowerableStmt for CallSite<F> {
    type F = F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
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

/// Group's IR
struct GroupBody<F> {
    name: String,
    /// Index in the original groups array.
    id: usize,
    io: (usize, usize),
    key: Option<GroupKeyInstance>,
    gates: IRStmt<IRAexpr<F>>,
    eq_constraints: IRStmt<IRAexpr<F>>,
    callsites: Vec<CallSite<F>>,
    lookups: IRStmt<IRAexpr<F>>,
}

impl<F: Field> GroupBody<F> {
    pub fn new(group: &Group, id: usize, ctx: GroupIRCtx<'_, '_, F>) -> anyhow::Result<Self> {
        let advice_io = group.advice_io();
        let instance_io = group.instance_io();

        let main = group.is_top_level();
        log::debug!("Lowering call-sites for group {:?}", group.name());
        let callsites = {
            group
                .children(ctx.groups)
                .into_iter()
                .enumerate()
                .map(|(call_no, (callee_id, callee))| {
                    CallSite::new(group, callee, callee_id, ctx, call_no)
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        log::debug!("Lowering gates for group {:?}", group.name());
        let gates = lower_gates(
            ctx.syn.gates(),
            &group.regions(),
            &ctx.patterns,
            &advice_io,
            &instance_io,
        )
        .and_then(scoped_exprs_to_aexpr)?;

        log::debug!(
            "Lowering inter region equality constraints for group {:?}",
            group.name()
        );
        let eq_constraints = scoped_exprs_to_aexpr(inter_region_constraints(
            select_equality_constraints(group, ctx),
            &advice_io,
            &instance_io,
        ))?;

        log::debug!("Lowering lookups for group {:?}", group.name());
        let lookups =
            codegen_lookup_invocations(ctx.syn, group.region_rows().as_slice(), ctx.lookup_cb)
                .and_then(scoped_exprs_to_aexpr)?;

        Ok(Self {
            id,
            io: (group.inputs().len(), group.outputs().len()),
            name: group.name().to_owned(),
            key: group.key(),
            callsites,
            gates,
            eq_constraints,
            lookups,
        })
    }

    pub fn is_main(&self) -> bool {
        self.key.is_none()
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

        lhs_key == rhs_key
            && lhs.io == rhs.io
            && Self::equivalent(&lhs.gates, &rhs.gates)
            && Self::equivalent(&lhs.eq_constraints, &rhs.eq_constraints)
            && Self::equivalent(&lhs.lookups, &rhs.lookups)
            && Self::equivalent(&lhs.callsites, &rhs.callsites)
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
        self.lookups.lower(l)
    }
}

/// Select the equality constraints that concern this group.
fn select_equality_constraints<F: Field>(
    group: &Group,
    ctx: GroupIRCtx<'_, '_, F>,
) -> Vec<EqConstraint<F>> {
    let region_indices: HashSet<_> = group
        .regions()
        .iter()
        .map(|r| *r.index().unwrap())
        .collect();
    let foreign_io: HashSet<_> = std::iter::chain(group.inputs(), group.outputs())
        .filter_map(|i| {
            if let GroupCell::Assigned(cell) = i {
                if !region_indices.contains(&cell.region_index) {
                    // Copy constraints use absolute rows but the labels have relative
                    // rows.
                    let abs_row =
                        cell.row_offset + ctx.regions_by_index[&cell.region_index].start()?;
                    return Some((cell.column, abs_row));
                }
            }

            None
        })
        .collect();
    // Selection criteria:
    //   - One of the columns' region is in the group's regions.
    //   - Is one of the IO cells and fall outside the group's regions.
    // Known limitation: If some cell in the group is constrained to a cell that is not
    // in the group and not marked as an input or output its going to generate a free variable.
    ctx.syn
        .constraints()
        .edges()
        .into_iter()
        .filter(|c| {
            match c {
                EqConstraint::AnyToAny(from, from_row, to, to_row) => {
                    if region_indices.contains(&from.index())
                        || region_indices.contains(&to.index())
                        || foreign_io.contains(&(*from, *from_row))
                        || foreign_io.contains(&(*to, *to_row))
                    {
                        return true;
                    }
                }
                // Can't annotate fixed columns with IO so these are just by region index
                EqConstraint::FixedToConst(column, _, _) => {
                    if region_indices.contains(&column.index()) {
                        return true;
                    }
                }
            }
            // If nothing matches then ignore.
            false
        })
        .collect()
}
