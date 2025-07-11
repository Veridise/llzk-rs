use anyhow::Result;

use crate::{
    expr::{traits::ExprLike, Expr},
    opt::Optimizer,
};

#[derive(Default)]
pub struct FoldExprsPass;

impl Optimizer<dyn ExprLike, Expr> for FoldExprsPass {
    fn optimize(&mut self, i: &dyn ExprLike) -> Result<Expr> {
        Ok(i.fold().unwrap_or_else(|| i.wrap()))
    }
}
