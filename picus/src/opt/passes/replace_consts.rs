use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::{
    expr::{traits::ConstraintExpr, Expr},
    felt::Felt,
    opt::MutOptimizer,
    stmt::traits::{CallLike as _, ConstraintLike as _, ExprArgs as _, MaybeCallLike as _},
    vars::{VarKind, VarStr},
    Module,
};

#[derive(Default)]
pub struct ReplaceKnownConstsPass;

type ReplacementSet = HashMap<VarStr, Felt>;

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

    fn find_call_output_vars(&self) -> HashSet<VarStr> {
        self.module
            .stmts()
            .iter()
            .flat_map(|stmt| stmt.as_call())
            .flat_map(|c| c.outputs().to_vec())
            .collect()
    }

    /// If (lhs, rhs) matches (VarExpr, ConstExpr) returns the var name and the const value.
    /// None otherwise. Only temporaries are matched.
    fn try_find_pattern(
        &self,
        lhs: Expr,
        rhs: Expr,
        call_output_vars: &HashSet<VarStr>,
    ) -> Option<(VarStr, Felt)> {
        // This is the least likely and least expensive check so do it first.
        let Some(f) = rhs.as_const() else {
            return None;
        };
        lhs.var_name()
            .filter(|var| {
                self.module
                    .vars()
                    .lookup_key(var)
                    .is_some_and(|k| k.is_temp())
                    && !call_output_vars.contains(var)
            })
            .map(|var| (var.clone(), f))
    }

    fn collect_replacements(&self) -> ReplacementSet {
        let mut set: HashMap<VarStr, HashSet<Felt>> = HashMap::new();
        let output_vars = self.find_call_output_vars();

        self.find_eq_constraint_exprs()
            .filter_map(|c| {
                self.try_find_pattern(c.lhs(), c.rhs(), &output_vars)
                    .or_else(|| self.try_find_pattern(c.rhs(), c.lhs(), &output_vars))
            })
            .for_each(|(var, felt)| {
                set.entry(var).or_default().insert(felt);
            });

        // In the rare case where a variable is equal to different values we conservatively remove
        // them from the set and let Picus complain at run time.
        set.retain(|_, values| values.len() == 1);

        set.into_iter()
            .map(|(k, values)| (k, values.into_iter().next().unwrap()))
            .collect()
    }

    fn replace_stmts(&mut self, replacement_set: &ReplacementSet) -> Result<()> {
        for stmt in self.module.stmts_mut() {
            stmt.args()
                .iter()
                .enumerate()
                .filter_map(|(idx, expr)| {
                    expr.replaced_by_const(replacement_set)
                        .map(|expr| (idx, expr))
                })
                .try_for_each(|(idx, new_arg)| stmt.replace_arg(idx, new_arg))?;
        }
        Ok(())
    }

    fn remove_tautos(&mut self) {
        fn is_tauto(expr: &dyn ConstraintExpr) -> bool {
            match (expr.lhs().as_const(), expr.rhs().as_const()) {
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

impl<K: VarKind + Copy> MutOptimizer<Module<K>> for ReplaceKnownConstsPass {
    fn optimize(&mut self, module: &mut Module<K>) -> Result<()> {
        let mut pass = PassImpl { module };

        let replacement_set = pass.collect_replacements();

        // Using the replacement set we replace all the variables in the module that need it with
        // the associated constant value.
        pass.replace_stmts(&replacement_set)?;

        // After replacing, some predicates will be A = A. We remove those here.
        pass.remove_tautos();

        Ok(())
    }
}
