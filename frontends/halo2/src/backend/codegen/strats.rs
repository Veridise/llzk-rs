pub mod inline {

    use std::borrow::Cow;

    use crate::{
        backend::codegen::{Codegen, CodegenStrategy},
        io::AllCircuitIO,
        ir::{expr::IRAexpr, IRCircuit, IRCtx},
    };
    use anyhow::Result;

    /// Code generation strategy that generates the all the code inside the main function.
    #[derive(Default)]
    #[allow(dead_code)]
    pub struct InlineConstraintsStrat {}

    impl CodegenStrategy for InlineConstraintsStrat {
        fn codegen<'c: 'st, 's, 'st, C>(
            &self,
            codegen: &C,
            ctx: &IRCtx,
            ir: &IRCircuit<IRAexpr>,
            //lookups: &dyn LookupCallbacks<C::F>,
            //gate_cbs: &dyn GateCallbacks<C::F>,
            //injector: &mut dyn crate::IRInjectCallback<C::F>,
        ) -> Result<()>
        where
            C: Codegen<'c, 'st>,
            //Row<'s, 's, C::F>: ResolversProvider<C::F> + 's,
            //RegionRow<'s, 's, 's, C::F>: ResolversProvider<C::F> + 's,
        {
            log::debug!(
                "Performing codegen with {} strategy",
                std::any::type_name_of_val(self)
            );

            log::debug!("Generating main body");
            let io = AllCircuitIO {
                advice: Cow::Borrowed(ctx.advice_io_of_group(ir.main().id())),
                instance: Cow::Borrowed(ctx.instance_io_of_group(ir.main().id())),
            };
            codegen.define_main_function_with_body(io, ir.groups().to_vec())

            //codegen.within_main(ir.main().all_io(), move |_| {
            //    Ok([ir.groups().to_vec()])
            //let patterns = load_patterns(gate_cbs);
            //
            //let mut stmts: Vec<IRStmt<_>> = vec![];
            //let top_level = syn
            //    .top_level_group()
            //    .ok_or_else(|| anyhow::anyhow!("Circuit synthesis is missing a top level group"))?;
            //let advice_io = top_level.advice_io();
            //let instance_io = top_level.instance_io();
            //for group in syn.groups() {
            //    // Do the region stmts first since backends may have more information about names for
            //    // cells there and some backends do not update the name and always use the first
            //    // one given.
            //
            //    log::debug!("Lowering gates");
            //    stmts.extend(
            //        lower_gates(
            //            syn.gates(),
            //            &group.regions(),
            //            &patterns,
            //            advice_io,
            //            instance_io,
            //            syn.fixed_query_resolver(),
            //        )
            //        .and_then(scoped_exprs_to_aexpr)?,
            //    );
            //    log::debug!("Lowering lookups");
            //    stmts.extend(
            //        codegen_lookup_invocations(
            //            syn,
            //            group.region_rows(syn.fixed_query_resolver()).as_slice(),
            //            lookups,
            //        )
            //        .and_then(scoped_exprs_to_aexpr)?,
            //    );
            //    log::debug!("Lowering inter region equality constraints");
            //    stmts.extend(scoped_exprs_to_aexpr(inter_region_constraints(
            //        syn.constraints().edges(),
            //        advice_io,
            //        instance_io,
            //        syn.fixed_query_resolver(),
            //    )));
            //
            //    for region in group.regions() {
            //        let index = region
            //            .index()
            //            .ok_or_else(|| anyhow::anyhow!("Region does not have an index"))?;
            //        let start = region.start().unwrap_or_default();
            //        if let Some(ir) = injector.inject(index, start) {
            //            stmts.push(crate::backend::codegen::lower_injected_ir(
            //                ir,
            //                region,
            //                advice_io,
            //                instance_io,
            //                syn.fixed_query_resolver(),
            //            )?);
            //        }
            //    }
            //}
            //Ok(stmts)
        }
    }
}

pub mod groups {

    use crate::halo2::groups::GroupKeyInstance;
    use crate::io::AllCircuitIO;
    use crate::ir::groups::GroupBody;
    use crate::ir::IRCtx;
    use crate::{
        backend::codegen::{Codegen, CodegenStrategy},
        ir::{
            equivalency::{EqvRelation, SymbolicEqv},
            expr::IRAexpr,
            IRCircuit,
        },
        utils,
    };
    use anyhow::Result;
    use std::borrow::Cow;
    use std::collections::{HashMap, HashSet};

    //mod body;
    //mod bounds;
    //mod callsite;

    /// Code generation strategy that write the code of each group in a separate function.
    #[derive(Default)]
    pub struct GroupConstraintsStrat {}

    impl CodegenStrategy for GroupConstraintsStrat {
        fn codegen<'c: 'st, 's, 'st, C>(
            &self,
            codegen: &C,
            ctx: &IRCtx,
            ir: &IRCircuit<IRAexpr>,
            //lookups: &dyn LookupCallbacks<C::F>,
            //gate_cbs: &dyn GateCallbacks<C::F>,
            //injector: &mut dyn crate::IRInjectCallback<C::F>,
        ) -> Result<()>
        where
            C: Codegen<'c, 'st>,
            //Row<'s, 's, C::F>: ResolversProvider<C::F> + 's,
            //RegionRow<'s, 's, 's, C::F>: ResolversProvider<C::F> + 's,
        {
            //log::debug!("Circuit synthesis has {} gates", syn.gates().len());
            //let patterns = load_patterns(gate_cbs);
            //let regions = region_data(syn)?;
            //let ctx = GroupIRCtx {
            //    groups: syn.groups(),
            //    regions_by_index: &regions,
            //    syn,
            //    patterns: &patterns,
            //    lookup_cb,
            //};
            //
            //log::debug!("Generating step 1 IR of region groups");
            //
            //let free_cells = free_cells::lift_free_cells_to_inputs(syn.groups(), ctx)?;
            //
            //let mut groups_ir = ctx
            //    .groups
            //    .iter()
            //    .enumerate()
            //    .map(|(id, g)| GroupBody::new(g, id, ctx, &free_cells[id], injector))
            //    .collect::<Result<Vec<_>, _>>()?;
            //
            //// Sanity check, only one group should be considered main.
            //assert_eq!(
            //    groups_ir.iter().filter(|g| g.is_main()).count(),
            //    1,
            //    "Only one main group is allowed"
            //);

            let mut groups_ir = ir.groups().to_vec();
            // Select leaders and generate the final names.
            // If the group was renamed its index will contain Some(_).
            let (leaders, updated_calldata) = select_leaders(&groups_ir);

            log::debug!("Leaders for the non-main groups: {leaders:?}");
            // Build the final list of IR and invoke codegen
            groups_ir.retain_mut(|g| {
                // Keep a group if its main or is in the leaders list.
                let keep = g.is_main() || leaders.contains(&g.id());
                if keep {
                    // If we are keeping it update the names if necessary.
                    update_names(g, &updated_calldata)
                }
                keep
            });

            // Create a function per group.
            for group in groups_ir {
                log::debug!("Group {group:#?}");
                let io = AllCircuitIO {
                    advice: Cow::Borrowed(ctx.advice_io_of_group(group.id())),
                    instance: Cow::Borrowed(ctx.instance_io_of_group(group.id())),
                };
                if group.is_main() {
                    log::debug!("Generating main body");
                    codegen.define_main_function_with_body(io, [group])?;
                } else {
                    log::debug!("Generating body of function {}", group.name());
                    let name = group.name().to_owned();

                    codegen.define_function_with_body(
                        &name,
                        io.input_count(),
                        io.output_count(),
                        |_, _, _| Ok([group]),
                    )?;
                }
            }
            Ok(())
        }
    }

    /// Organizes the groups by their key. Each group contains a list with the index to the group.
    pub fn organize_groups_by_key(
        groups: &[GroupBody<IRAexpr>],
    ) -> HashMap<GroupKeyInstance, Vec<usize>> {
        let mut groups_by_key: HashMap<_, Vec<_>> = HashMap::new();
        for group in groups {
            if group.is_main() {
                log::debug!("Group {} is main. Skipping...", group.id());
                continue;
            }
            groups_by_key
                .entry(group.key().expect("Non main group needs a key"))
                .or_default()
                .push(group.id());
            log::debug!("Inserting group {} with key {:?}", group.id(), group.key());
        }
        groups_by_key
    }

    /// Find the leaders of each equivalence class in the groups and annotate the required renames
    fn select_leaders(groups_ir: &[GroupBody<IRAexpr>]) -> (Vec<usize>, Vec<Option<String>>) {
        // Separate the groups by their key.
        let groups_by_key = organize_groups_by_key(groups_ir);
        log::debug!("Groups: {groups_by_key:?}");
        let mut leaders = vec![];
        // For each group annotate its new name if it needs to be renamed.
        let mut updated_calldata: Vec<Option<String>> = vec![None; groups_ir.len()];
        // Avoids duplicating names
        let mut used_names: HashSet<String> = HashSet::default();
        // For each set of groups with the same key we create equivalence classes and select a
        // leader for each class.
        let mut eqv_class = disjoint::DisjointSet::new();
        // Keeps track of the inserted elements in the equivalence class.
        let eqv_class_ids: Vec<_> = (0..groups_ir.len())
            .map(|_| eqv_class.add_singleton())
            .collect();
        for groups in groups_by_key.values() {
            // Find the equivalence classes.
            for (i, j) in utils::product(groups.as_slice(), groups.as_slice()) {
                if *i == *j {
                    continue;
                }
                let lhs = &groups_ir[*i];
                let rhs = &groups_ir[*j];
                if SymbolicEqv::equivalent(lhs, rhs) {
                    eqv_class.join(eqv_class_ids[*i], eqv_class_ids[*j]);
                }
            }
        }

        // Flip the mapping between ids to recover them.
        let eqv_class_ids: HashMap<_, _> = eqv_class_ids
            .into_iter()
            .enumerate()
            .map(|(n, id)| (id, n))
            .collect();

        // For each group eqv class select a leader and annotate what groups need to be updated.
        for (n, set) in eqv_class.sets().into_iter().enumerate() {
            debug_assert!(!set.is_empty());
            let set: Vec<_> = set.into_iter().map(|id| eqv_class_ids[&id]).collect();
            // We arbitrarily chose the leader to be the first element.
            leaders.push(set[0]);
            let leader = groups_ir.get(set[0]).unwrap();
            let name = fresh_group_name(leader.name(), &mut used_names, n);
            for update in &set {
                updated_calldata[*update] = Some(name.clone());
            }
        }

        (leaders, updated_calldata)
    }

    /// Updates the name of the group and the names of the functions each callsite references.
    fn update_names(group: &mut GroupBody<IRAexpr>, updated_calldata: &[Option<String>]) {
        if let Some(name) = &updated_calldata[group.id()] {
            *group.name_mut() = name.clone();
        }
        for callsite in group.callsites_mut() {
            if let Some(name) = &updated_calldata[callsite.callee_id()] {
                callsite.set_name(name.clone());
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
}
