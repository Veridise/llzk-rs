use crate::{
    display::{TextRepresentable, TextRepresentation},
    expr::{impls::BinaryExpr, Expr},
    felt::Felt,
    stmt::traits::ConstraintLike,
};

use super::{OpFolder, OpLike};

#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub enum Boolean {
    And,
    Or,
    Implies,
    Iff,
}

impl Boolean {
    fn fold_side(&self, lhs: &Expr, rhs: &Expr) -> Option<Expr> {
        log::debug!("Trying to fold ({self:?}  {lhs:?}  {rhs:?})");
        lhs.constraint_expr()
            .and_then(move |clhs| match self {
                // T && x == x
                Boolean::And if clhs.is_constant_true() => Some(rhs.clone()),
                // F && x == F
                Boolean::And if clhs.is_constant_false() => Some(lhs.clone()),
                // T || x == T
                Boolean::Or if clhs.is_constant_true() => Some(lhs.clone()),
                // F || x == x
                Boolean::Or if clhs.is_constant_false() => Some(rhs.clone()),
                _ => None,
            })
            .inspect(|e| log::debug!("Folded to {e:?}"))
    }
}

impl OpFolder for Boolean {
    fn fold(&self, lhs: Expr, rhs: Expr, _prime: &Felt) -> Option<Expr> {
        self.fold_side(&lhs, &rhs).or_else(|| {
            self.flip(&lhs, &rhs)
                .and_then(|flipped| flipped.op().fold_side(&flipped.lhs(), &flipped.rhs()))
        })
    }

    fn commutative(&self) -> bool {
        true
    }

    fn flip(&self, lhs: &Expr, rhs: &Expr) -> Option<BinaryExpr<Self>> {
        match self {
            Boolean::And => Some(BinaryExpr::new(Self::And, rhs.clone(), lhs.clone())),
            Boolean::Or => Some(BinaryExpr::new(Self::Or, rhs.clone(), lhs.clone())),
            _ => None,
        }
    }
}

impl TextRepresentable for Boolean {
    fn to_repr(&self) -> TextRepresentation<'_> {
        TextRepresentation::atom(match self {
            Boolean::And => "&&",
            Boolean::Or => "||",
            Boolean::Implies => "=>",
            Boolean::Iff => "<=>",
        })
    }

    fn width_hint(&self) -> usize {
        match self {
            Boolean::And => 2,
            Boolean::Or => 2,
            Boolean::Implies => 2,
            Boolean::Iff => 3,
        }
    }
}

impl OpLike for Boolean {
    fn extraible(&self) -> bool {
        true
    }
}
