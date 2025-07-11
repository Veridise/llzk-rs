use std::collections::HashMap;

use anyhow::Result;
use disjoint::DisjointSetVec;

use crate::{
    expr::traits::ConstraintExpr,
    opt::MutOptimizer,
    stmt::traits::{ConstraintLike as _, ExprArgs as _},
    vars::{VarKind, VarStr},
    Module,
};

#[derive(Default)]
pub struct ConsolidateVarNamesPass;

type RenameSet = HashMap<VarStr, VarStr>;

struct PassImpl<'a, K: VarKind> {
    module: &'a mut Module<K>,
}

impl<'m, K: VarKind + Copy> PassImpl<'m, K> {
    fn find_eq_constraint_exprs(&self) -> impl Iterator<Item = &dyn ConstraintExpr> {
        self.module
            .stmts()
            .iter()
            .filter_map(|stmt| stmt.constraint_expr())
            .filter(|c| c.is_eq())
    }

    fn compute_eqv_classes(&self) -> DisjointSetVec<VarStr> {
        let (set, _) = self
            .find_eq_constraint_exprs()
            .filter_map(|c| Some((c.lhs().var_name()?.clone(), c.rhs().var_name()?.clone())))
            .fold(
                (
                    DisjointSetVec::<VarStr>::new(),
                    HashMap::<VarStr, usize>::new(), // Used to avoid inserting twice the same var
                ),
                |(mut set, mut seen), (lhs, rhs)| {
                    let lhs = *seen.entry(lhs.clone()).or_insert_with(|| set.push(lhs));
                    let rhs = *seen.entry(rhs.clone()).or_insert_with(|| set.push(rhs));
                    set.join(lhs, rhs);

                    (set, seen)
                },
            );
        set
    }

    fn find_vars<'a>(
        &self,
        class: &[usize],
        set: &'a DisjointSetVec<VarStr>,
    ) -> Result<Vec<(K, &'a VarStr)>> {
        class
            .iter()
            .map(|idx| set.get(*idx))
            .map(|var| {
                var.and_then(|var| self.module.vars().lookup_key(var).map(|k| (*k, var)))
                    .ok_or_else(|| anyhow::anyhow!("Variable name not in environment: {var:?}"))
            })
            .collect::<Result<Vec<_>, _>>()
    }

    fn collect_temps<'a>(&self, vars: &[(K, &'a VarStr)]) -> Vec<(K, &'a VarStr)> {
        vars.iter()
            .copied()
            .filter(|(k, _)| k.is_temp())
            .collect::<Vec<_>>()
    }

    fn select_leader<'a>(&self, vars: &[(K, &'a VarStr)]) -> Option<(K, &'a VarStr)> {
        vars.iter()
            .filter(|(k, _)| !k.is_temp())
            .copied()
            .fold(None, |acc, (k, v)| {
                debug_assert!(!k.is_temp());
                // If the accumulator is a temp we pick the non-temp regardless.
                if acc.is_none() {
                    return Some((k, v));
                }
                let (acc_k, _) = acc.unwrap();
                // If the accumulator is an output and we have an input pick that.
                if acc_k.is_output() && k.is_input() {
                    return Some((k, v));
                }
                // Otherwise just keep the current accumulator.
                acc
            })
    }

    fn handle_eqv_class(
        &self,
        class: Vec<usize>,
        set: &DisjointSetVec<VarStr>,
    ) -> Result<impl Iterator<Item = (VarStr, VarStr)>> {
        assert!(!class.is_empty());
        // Collect all the vars that are used in A = B constraints
        let vars = self.find_vars(&class, set)?;

        // Gather the temporaries, which are the ones that can be renamed
        let temps = self.collect_temps(&vars);

        // Select a leader from the group (order of priority: inputs, outputs, and then a
        // temporary). We fold here so that if the list of non-temp variables in the class is
        // empty we just use the fist temp we generated.
        let leader = self.select_leader(&vars).or_else(|| temps.get(0).copied());

        Ok(temps
            .into_iter()
            .zip(leader.into_iter().cycle())
            .filter(|(to_rename, leader)| to_rename != leader)
            .map(|((_, renamed), (_, with))| (renamed.clone(), with.clone()))
            .inspect(|(renamed, with)| log::debug!("Variable {renamed} will be renamed to {with}")))
    }

    fn compute_rename_set(&self) -> Result<RenameSet> {
        let ec = self.compute_eqv_classes();

        Ok(ec
            .indices()
            .sets()
            .into_iter()
            .map(|class| self.handle_eqv_class(class, &ec))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flat_map(|x| x)
            .collect::<RenameSet>())
    }

    fn rename_stmts(&mut self, rename_set: &RenameSet) -> Result<()> {
        for stmt in self.module.stmts_mut() {
            for (idx, new_arg) in stmt
                .args()
                .iter()
                .map(|expr| expr.renamed(rename_set).unwrap_or(expr.clone()))
                .enumerate()
            {
                stmt.replace_arg(idx, new_arg)?;
            }
        }
        Ok(())
    }

    fn remove_tautos(&mut self) {
        fn is_tauto(expr: &dyn ConstraintExpr) -> bool {
            match (expr.lhs().var_name(), expr.rhs().var_name()) {
                (Some(lhs), Some(rhs)) => lhs == rhs,
                _ => false,
            }
        }

        self.module.remove_stmt_if(|stmt| {
            stmt.constraint_expr()
                .map(is_tauto)
                .inspect(|remove| {
                    if *remove {
                        log::debug!("Removing {stmt:?}")
                    }
                })
                .unwrap_or_default()
        })
    }
}

impl<K: VarKind + Copy> MutOptimizer<Module<K>> for ConsolidateVarNamesPass {
    fn optimize(&mut self, module: &mut Module<K>) -> Result<()> {
        let mut pass = PassImpl { module };

        // Compute from the module's statements what variables are aliases
        // and from there derive a mapping of var names that can be renamed to
        // the name that they can reuse.
        let rename_set = pass.compute_rename_set()?;

        // Using the rename set we rename all the variables in the module that need it.
        pass.rename_stmts(&rename_set)?;

        // After the renaming some predicates will be A = A. We remove those here.
        pass.remove_tautos();

        Ok(())
    }
}
