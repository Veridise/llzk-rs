
use crate::ir::{opt::langs::bexpr::BexprLang, CmpOp};

#[allow(dead_code)]
pub enum Stmt<A>
where
    A: std::fmt::Debug + Clone + Eq + Ord + std::hash::Hash,
{
    Constr(CmpOp, A, A),
    Assert(BexprLang<A>),
}
