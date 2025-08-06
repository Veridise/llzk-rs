use crate::{
    display::{TextRepresentable, TextRepresentation},
    expr::{self, impls::BinaryExpr, Expr},
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

fn zip_option<L, R>(lhs: Option<L>, rhs: Option<R>) -> Option<(L, R)> {
    lhs.and_then(|lhs| rhs.map(|rhs| (lhs, rhs)))
}

impl ConstraintKind {
    fn fold_impl(&self, lhs: &Expr, rhs: &Expr) -> Option<bool> {
        zip_option(lhs.as_const(), rhs.as_const()).map(|(lhs, rhs)| match self {
            ConstraintKind::Lt => lhs < rhs,
            ConstraintKind::Le => lhs <= rhs,
            ConstraintKind::Gt => lhs > rhs,
            ConstraintKind::Ge => lhs >= rhs,
            ConstraintKind::Eq => lhs == rhs,
            ConstraintKind::Ne => lhs != rhs,
        })
    }
}

impl OpFolder for ConstraintKind {
    fn fold(&self, lhs: Expr, rhs: Expr, _prime: &Felt) -> Option<Expr> {
        self.fold_impl(&lhs, &rhs)
            .map(|b| if b { expr::r#true() } else { expr::r#false() })
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
            ConstraintKind::Eq => None,
            ConstraintKind::Ne => None,
        }
    }
}

impl TextRepresentable for ConstraintKind {
    fn to_repr(&self) -> TextRepresentation<'_> {
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
