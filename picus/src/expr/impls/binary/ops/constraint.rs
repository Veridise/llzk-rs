use crate::{
    display::{TextRepresentable, TextRepresentation},
    expr::{impls::BinaryExpr, Expr},
    felt::Felt,
};

use super::{OpFolder, OpLike};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstraintKind {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl OpLike for ConstraintKind {
    fn extraible(&self) -> bool {
        false
    }
}

impl OpFolder for ConstraintKind {
    fn fold(&self, _lhs: Expr, _rhs: Expr, _prime: &Felt) -> Option<Expr> {
        None
    }

    fn commutative(&self) -> bool {
        matches!(self, ConstraintKind::Eq)
    }

    fn flip(&self, lhs: &Expr, rhs: &Expr) -> Option<BinaryExpr<Self>> {
        match self {
            ConstraintKind::Lt => Some(BinaryExpr::new(Self::Ge, rhs.clone(), lhs.clone())),
            ConstraintKind::Le => Some(BinaryExpr::new(Self::Gt, rhs.clone(), lhs.clone())),
            ConstraintKind::Gt => Some(BinaryExpr::new(Self::Le, rhs.clone(), lhs.clone())),
            ConstraintKind::Ge => Some(BinaryExpr::new(Self::Lt, rhs.clone(), lhs.clone())),
            ConstraintKind::Eq => Some(BinaryExpr::new(Self::Eq, rhs.clone(), lhs.clone())),
            ConstraintKind::Ne => Some(BinaryExpr::new(Self::Ne, rhs.clone(), lhs.clone())),
        }
    }
}

impl TextRepresentable for ConstraintKind {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::atom(match self {
            ConstraintKind::Lt => "<",
            ConstraintKind::Le => "<=",
            ConstraintKind::Gt => ">",
            ConstraintKind::Ge => ">=",
            ConstraintKind::Eq => "=",
            ConstraintKind::Ne => "!=",
        })
    }

    fn width_hint(&self) -> usize {
        match self {
            ConstraintKind::Lt | ConstraintKind::Gt | ConstraintKind::Eq => 1,
            ConstraintKind::Le | ConstraintKind::Ge | ConstraintKind::Ne => 2,
        }
    }
}
