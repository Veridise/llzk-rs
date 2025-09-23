use egg::{Id, LanguageChildren as _};

use crate::{
    backend::func::FuncIO,
    halo2::Challenge,
    ir::{expr::Felt, opt::langs::bexpr::BexprLang, CmpOp},
};

#[allow(dead_code)]
pub enum Stmt<A>
where
    A: std::fmt::Debug + Clone + Eq + Ord + std::hash::Hash,
{
    Constr(CmpOp, A, A),
    Assert(BexprLang<A>),
}
