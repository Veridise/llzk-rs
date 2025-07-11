use std::collections::HashMap;

use crate::{
    display::TextRepresentable, felt::Felt, opt::Optimizer, stmt::traits::ConstraintLike,
    vars::VarStr,
};

use super::Expr;
use anyhow::Result;

pub trait MaybeVarLike {
    fn var_name(&self) -> Option<&VarStr>;

    fn renamed(&self, map: &HashMap<VarStr, VarStr>) -> Option<Expr>;
}

pub trait ConstraintEmitter {
    fn emit(&mut self, lhs: Expr, rhs: Expr);
}

pub trait WrappedExpr {
    fn wrap(&self) -> Expr;
}

pub trait ExprSize {
    /// Returns the number of nodes in the expression.
    fn size(&self) -> usize;

    /// True if the expression can be extracted to a temporary
    fn extraible(&self) -> bool;

    fn args(&self) -> Vec<Expr>;

    fn replace_args(&self, args: &[Option<Expr>]) -> Result<Option<Expr>>;
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

pub trait ConstraintExpr {
    fn is_eq(&self) -> bool;

    fn lhs(&self) -> Expr;

    fn rhs(&self) -> Expr;
}

/// Marker interface for a Picus expression.
pub trait ExprLike:
    ExprSize
    + ConstantFolding
    + TextRepresentable
    + WrappedExpr
    + MaybeVarLike
    + std::fmt::Debug
    + ConstraintLike
{
}
