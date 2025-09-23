use egg::{FromOp, Id, Language, LanguageChildren as _};

use crate::{backend::func::FuncIO, halo2::Challenge, ir::expr::Felt};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AexprLang {
    Lit(Felt),
    Var(FuncIO),
    Chall(Challenge),
    Neg(Id),
    Sum([Id; 2]),
    Product([Id; 2]),
}

impl std::fmt::Display for AexprLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AexprLang::Lit(felt) => write!(f, "{felt}"),
            AexprLang::Var(func_io) => write!(f, "{func_io}"),
            AexprLang::Chall(challenge) => write!(f, "chall{}", challenge.index()),
            AexprLang::Neg(_) => write!(f, "-"),
            AexprLang::Sum(_) => write!(f, "+"),
            AexprLang::Product(_) => write!(f, "*"),
        }
    }
}

impl FromOp for AexprLang {
    type Error = anyhow::Error;

    fn from_op(_op: &str, _children: Vec<Id>) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl Language for AexprLang {
    type Discriminant = std::mem::Discriminant<Self>;

    fn discriminant(&self) -> Self::Discriminant {
        std::mem::discriminant(self)
    }

    fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (AexprLang::Lit(lhs), AexprLang::Lit(rhs)) => lhs == rhs,
            (AexprLang::Var(lhs), AexprLang::Var(rhs)) => lhs == rhs,
            (AexprLang::Chall(lhs), AexprLang::Chall(rhs)) => lhs == rhs,
            (AexprLang::Neg(_), AexprLang::Neg(_)) => true,
            (AexprLang::Sum(_), AexprLang::Sum(_)) => true,
            (AexprLang::Product(_), AexprLang::Product(_)) => true,
            _ => false,
        }
    }

    fn children(&self) -> &[Id] {
        match self {
            AexprLang::Lit(_) | AexprLang::Var(_) | AexprLang::Chall(_) => &[],
            AexprLang::Neg(id) => id.as_slice(),
            AexprLang::Sum(ids) | AexprLang::Product(ids) => ids,
        }
    }

    fn children_mut(&mut self) -> &mut [Id] {
        match self {
            AexprLang::Lit(_) | AexprLang::Var(_) | AexprLang::Chall(_) => &mut [],
            AexprLang::Neg(id) => id.as_mut_slice(),
            AexprLang::Sum(ids) | AexprLang::Product(ids) => ids,
        }
    }
}

impl PartialOrd for AexprLang {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AexprLang {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            // AexprLang::Lit
            (AexprLang::Lit(lhs), AexprLang::Lit(rhs)) => lhs.cmp(rhs),
            (AexprLang::Lit(_), _) => std::cmp::Ordering::Less,
            // AexprLang::Var
            (AexprLang::Var(_), AexprLang::Lit(_)) => std::cmp::Ordering::Greater,
            (AexprLang::Var(lhs), AexprLang::Var(rhs)) => lhs.cmp(rhs),
            (AexprLang::Var(_), _) => std::cmp::Ordering::Less,
            // AexprLang::Chall
            (AexprLang::Chall(_), AexprLang::Lit(_) | AexprLang::Var(_)) => {
                std::cmp::Ordering::Greater
            }
            (AexprLang::Chall(lhs), AexprLang::Chall(rhs)) => lhs.index().cmp(&rhs.index()),
            (AexprLang::Chall(_), _) => std::cmp::Ordering::Less,
            // AexprLang::Neg
            (AexprLang::Neg(_), AexprLang::Lit(_) | AexprLang::Var(_) | AexprLang::Chall(_)) => {
                std::cmp::Ordering::Greater
            }
            (AexprLang::Neg(lhs), AexprLang::Neg(rhs)) => lhs.cmp(&rhs),
            (AexprLang::Neg(_), _) => std::cmp::Ordering::Less,
            // AexprLang::Sum
            (
                AexprLang::Sum(_),
                AexprLang::Lit(_) | AexprLang::Var(_) | AexprLang::Chall(_) | AexprLang::Neg(_),
            ) => std::cmp::Ordering::Greater,
            (AexprLang::Sum(lhs), AexprLang::Sum(rhs)) => lhs.cmp(&rhs),
            (AexprLang::Sum(_), AexprLang::Product(_)) => std::cmp::Ordering::Less,
            // AexprLang::Product
            (AexprLang::Product(lhs), AexprLang::Product(rhs)) => lhs.cmp(&rhs),
            (AexprLang::Product(_), _) => std::cmp::Ordering::Greater,
        }
    }
}
