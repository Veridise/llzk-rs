use egg::{Id, LanguageChildren as _};

use crate::ir::CmpOp;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BexprLang<A>
where
    A: std::fmt::Debug + Clone + Eq + Ord + std::hash::Hash,
{
    Lit(bool),
    Aexpr(A),
    Cmp(CmpOp, [Id; 2]),
    Not(Id),
    And([Id; 2]),
    Or([Id; 2]),
}

impl<A> From<A> for BexprLang<A>
where
    A: std::fmt::Debug + Clone + Eq + Ord + std::hash::Hash,
{
    fn from(value: A) -> Self {
        Self::Aexpr(value)
    }
}

impl<A> egg::Language for BexprLang<A>
where
    A: egg::Language + std::fmt::Debug,
{
    type Discriminant = std::mem::Discriminant<Self>;

    fn discriminant(&self) -> Self::Discriminant {
        std::mem::discriminant(self)
    }

    fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (BexprLang::Lit(lhs), BexprLang::Lit(rhs)) => lhs == rhs,
            (BexprLang::Aexpr(lhs), BexprLang::Aexpr(rhs)) => lhs.matches(rhs),
            (BexprLang::Cmp(lhs, _), BexprLang::Cmp(rhs, _)) => lhs == rhs,
            (BexprLang::Not(_), BexprLang::Not(_)) => true,
            (BexprLang::And(_), BexprLang::And(_)) => true,
            (BexprLang::Or(_), BexprLang::Or(_)) => true,
            _ => false,
        }
    }

    fn children(&self) -> &[Id] {
        match self {
            BexprLang::Lit(_) => &[],
            BexprLang::Aexpr(expr) => expr.children(),
            BexprLang::Cmp(_, ids) | BexprLang::And(ids) | BexprLang::Or(ids) => ids,
            BexprLang::Not(id) => id.as_slice(),
        }
    }

    fn children_mut(&mut self) -> &mut [Id] {
        match self {
            BexprLang::Lit(_) => &mut [],
            BexprLang::Aexpr(expr) => expr.children_mut(),
            BexprLang::Cmp(_, ids) | BexprLang::And(ids) | BexprLang::Or(ids) => ids,
            BexprLang::Not(id) => id.as_mut_slice(),
        }
    }
}
