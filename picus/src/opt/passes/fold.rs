use std::marker::PhantomData;

use anyhow::Result;

use crate::{
    expr::{traits::ExprLike, Expr},
    felt::IntoPrime,
    opt::Optimizer,
};

#[derive(Default)]
pub struct FoldExprsPass<P>(PhantomData<P>)
where
    P: IntoPrime;

impl<P: IntoPrime> Optimizer<dyn ExprLike, Expr> for FoldExprsPass<P> {
    fn optimize(&mut self, i: &dyn ExprLike) -> Result<Expr> {
        Ok(i.fold(&P::prime()).unwrap_or_else(|| i.wrap()))
    }
}
