pub mod inline {

    use crate::{
        backend::codegen::{Codegen, CodegenStrategy},
        ir::{IRCtx, ResolvedIRCircuit},
    };
    use anyhow::Result;

    /// Code generation strategy that generates the all the code inside the main function.
    #[derive(Default)]
    pub struct InlineConstraintsStrat {}

    impl CodegenStrategy for InlineConstraintsStrat {
        fn codegen<'c: 'st, 's, 'st, C>(
            &self,
            codegen: &C,
            ctx: &IRCtx,
            ir: &ResolvedIRCircuit,
        ) -> Result<()>
        where
            C: Codegen<'c, 'st>,
        {
            log::debug!(
                "Performing codegen with {} strategy",
                std::any::type_name_of_val(self)
            );

            log::debug!("Generating main body");
            let main_id = ir.main().id();
            codegen.define_main_function_with_body(
                ctx.advice_io_of_group(main_id),
                ctx.instance_io_of_group(main_id),
                ir.groups().to_vec(),
            )
        }
    }
}

pub mod groups {

    use crate::ir::IRCtx;
    use crate::ir::expr::IRAexpr;
    use crate::ir::groups::GroupBody;
    use crate::synthesis::groups::GroupKey;
    use crate::{
        backend::codegen::{Codegen, CodegenStrategy},
        ir::{
            ResolvedIRCircuit,
            equivalency::{EqvRelation, SymbolicEqv},
        },
        utils,
    };
    use anyhow::Result;
    use std::collections::{HashMap, HashSet};

    /// Code generation strategy that write the code of each group in a separate function.
    #[derive(Default)]
    pub struct GroupConstraintsStrat {}

    impl CodegenStrategy for GroupConstraintsStrat {
        fn codegen<'c: 'st, 's, 'st, C>(
            &self,
            codegen: &C,
            ctx: &IRCtx,
            ir: &ResolvedIRCircuit,
        ) -> Result<()>
        where
            C: Codegen<'c, 'st>,
        {
            let mut groups_ir = ir.groups().to_vec();
            // Select leaders and generate the final names.
            // If the group was renamed its index will contain Some(_).
            let (leaders, updated_calldata) = select_leaders(&groups_ir);

            log::debug!("Leaders for the non-main groups: {leaders:?}");
            log::debug!("Updated calldata: {updated_calldata:?}");
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

            let mut err_count = 0;
            for group in &groups_ir {
                log::debug!("Validating group \"{}\"", group.name());
                let (validation_status, validation_errors) = group.validate(ir.groups());
                if validation_status.is_err() {
                    err_count += validation_errors.len();
                    for err in validation_errors {
                        log::error!("Codegen error: {err}");
                    }
                }
            }
            if err_count > 0 {
                anyhow::bail!("Codegen failed due to {err_count} validation errors");
            }
            // Create a function per group.
            for group in groups_ir {
                log::debug!("Generating code for group \"{}\"", group.name());

                let advice_io = ctx.advice_io_of_group(group.id());
                let instance_io = ctx.instance_io_of_group(group.id());
                if group.is_main() {
                    log::debug!("Generating main body");
                    codegen.define_main_function_with_body(advice_io, instance_io, [group])?;
                } else {
                    log::debug!("Generating body of function {}", group.name());
                    let name = group.name().to_owned();

                    codegen.define_function_with_body(
                        &name,
                        advice_io.inputs_count() + instance_io.inputs_count(),
                        advice_io.outputs_count() + instance_io.outputs_count(),
                        |_, _, _| Ok([group]),
                    )?;
                }
            }
            Ok(())
        }
    }

    /// Organizes the groups by their key. Each group contains a list with the index to the group.
    pub fn organize_groups_by_key(groups: &[GroupBody<IRAexpr>]) -> HashMap<GroupKey, Vec<usize>> {
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
    fn select_leaders(
        groups_ir: &[GroupBody<IRAexpr>],
    ) -> (Vec<usize>, Vec<Option<(usize, String)>>) {
        // Separate the groups by their key.
        let groups_by_key = organize_groups_by_key(groups_ir);
        log::debug!("Groups: {groups_by_key:?}");
        let mut leaders = vec![];
        // For each group annotate its new name if it needs to be renamed.
        let mut updated_calldata: Vec<Option<(usize, String)>> = vec![None; groups_ir.len()];
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
            let leader_id = set[0];
            leaders.push(leader_id);
            let leader = groups_ir.get(leader_id).unwrap();
            let name = fresh_group_name(leader.name(), &mut used_names, n);
            for update in &set {
                updated_calldata[*update] = Some((leader_id, name.clone()));
            }
        }

        (leaders, updated_calldata)
    }

    /// Updates the name of the group and the names of the functions each callsite references.
    fn update_names(group: &mut GroupBody<IRAexpr>, updated_calldata: &[Option<(usize, String)>]) {
        if let Some((id, name)) = &updated_calldata[group.id()] {
            *group.name_mut() = name.clone();
            group.set_id(*id);
        }
        for callsite in group.callsites_mut() {
            if let Some((id, name)) = &updated_calldata[callsite.callee_id()] {
                callsite.set_callee_id(*id);
                callsite.set_name(name.clone());
            }
        }
    }

    /// Finds a version of the given name that is fresh.
    fn fresh_group_name(name: &str, used_names: &mut HashSet<String>, n: usize) -> String {
        // Create a lazy iterator with the input name and every rename and then consume it until we get
        // a valid name.
        let name = [name.to_owned()]
            .into_iter()
            .chain((n..).map(|n| format!("{name}{n}")))
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
