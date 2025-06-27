use crate::felt::Felt;

use super::Expr;

pub trait ExprSize {
    /// Returns the number of nodes in the expression.
    fn size(&self) -> usize;
}

pub trait ConstantFolding {
    /// If the expression folded to a constant returns Some(const), otherwise returns None
    fn as_const(&self) -> Option<Felt>;

    /// If the expression folded returns Some(expr), otherwise returns None
    fn fold(&self) -> Option<Expr>;

    /// Returns true if the expression folds to a constant 1.
    fn is_one(&self) -> bool {
        if let Some(n) = self.as_const() {
            return n.is_one();
        }
        false
    }

    /// Returns true if the expression folds to a constant 0.
    fn is_zero(&self) -> bool {
        if let Some(n) = self.as_const() {
            return n.is_zero();
        }
        false
    }
}

/// Marker interface for a Picus expression.
pub trait ExprLike: ExprSize + std::fmt::Display + ConstantFolding {}
