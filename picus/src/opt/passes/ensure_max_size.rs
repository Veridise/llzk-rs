use anyhow::{anyhow, Result};

use crate::{
    expr::{
        self,
        traits::{ConstraintEmitter, ExprLike},
        Expr,
    },
    opt::{MutOptimizer, Optimizer},
    stmt::{self, Stmt},
    vars::{Temp, VarStr},
    Module,
};

pub struct EnsureMaxExprSizePass<C> {
    limit: usize,
    ctx: C,
}

impl<C> From<(usize, C)> for EnsureMaxExprSizePass<C> {
    fn from((limit, ctx): (usize, C)) -> Self {
        Self { limit, ctx }
    }
}

impl ConstraintEmitter for Vec<Stmt> {
    fn emit(&mut self, lhs: Expr, rhs: Expr) {
        self.push(stmt::constrain(expr::eq(&lhs, &rhs)))
    }
}

impl<K: Temp<Ctx = C>, C: Copy> MutOptimizer<Module<K>> for EnsureMaxExprSizePass<C> {
    fn optimize(&mut self, t: &mut Module<K>) -> Result<()> {
        let temporaries = [K::temp(self.ctx)]
            .into_iter()
            .cycle()
            .map(|k| -> VarStr { k.into() })
            .enumerate()
            .map(|(idx, t)| -> Result<VarStr> { format!("{t}{idx}").try_into() });
        let mut new_constraints = vec![];
        let mut r#impl = EnsureMaxExprSizePassImpl {
            limit: self.limit,
            emitter: &mut new_constraints,
            temporaries,
        };

        MutOptimizer::optimize(&mut r#impl, t)?;

        t.add_stmts(&new_constraints);
        Ok(())
    }
}

struct EnsureMaxExprSizePassImpl<'a, E, T> {
    limit: usize,
    emitter: &'a mut E,
    temporaries: T,
}

impl<E, T> Optimizer<dyn ExprLike, Expr> for EnsureMaxExprSizePassImpl<'_, E, T>
where
    E: ConstraintEmitter,
    T: Iterator<Item = Result<VarStr>>,
{
    /// If the expression's size is larger than the threshold
    /// replaces the expression with a temporary and emit a constraint that
    /// equates that fresh temporary with the expression.
    /// If not returns itself.
    fn optimize(&mut self, expr: &(dyn ExprLike)) -> Result<Expr> {
        if expr.size() < self.limit {
            return Ok(expr.wrap());
        }
        let args: Vec<Option<Expr>> = expr
            .args()
            .iter()
            .map(|arg| Optimizer::<dyn ExprLike, Expr>::optimize(self, arg.as_ref()))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(Some)
            .collect();
        let transformed = expr.replace_args(&args)?;

        let expr = match &transformed {
            Some(expr) => expr.as_ref(),
            None => expr,
        };

        if expr.size() < self.limit || !expr.extraible() {
            return Ok(expr.wrap());
        }
        let temp = expr::known_var(
            &self
                .temporaries
                .next()
                .ok_or_else(|| anyhow!("Temporaries generator is exhausted"))??,
        );
        self.emitter.emit(temp.clone(), expr.wrap());
        Ok(temp)
    }
}
