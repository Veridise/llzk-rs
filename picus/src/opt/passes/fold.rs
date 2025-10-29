use anyhow::Result;

use crate::{
    expr::{traits::ExprLike, Expr},
    felt::Felt,
    opt::{MutOptimizer, Optimizer},
    vars::VarKind,
    Program,
};

#[derive(Default, Debug)]
pub struct FoldExprsPass;

impl<K: VarKind + Copy> MutOptimizer<Program<K>> for FoldExprsPass {
    fn optimize(&mut self, t: &mut Program<K>) -> Result<()> {
        let prime = t.prime().clone();
        let mut inner = FoldExprsPassImpl(prime);
        let opt: &mut dyn MutOptimizer<Program<K>> = &mut inner;
        opt.optimize(t)
    }
}

#[derive(Debug)]
struct FoldExprsPassImpl(Felt);

impl Optimizer<dyn ExprLike, Expr> for FoldExprsPassImpl {
    fn optimize(&mut self, i: &dyn ExprLike) -> Result<Expr> {
        Ok(i.fold(&self.0).unwrap_or_else(|| i.wrap()))
    }
}
