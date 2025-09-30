use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use crate::{display::TextRepresentable, felt::Felt, stmt::traits::ConstraintLike, vars::VarStr};

use super::{util::map_consts, Expr, ExprHash};
use anyhow::Result;

pub trait MaybeVarLike {
    fn var_name(&self) -> Option<&VarStr>;

    fn renamed(&self, map: &HashMap<VarStr, VarStr>) -> Option<Expr>;

    fn free_vars(&self) -> HashSet<&VarStr>;
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
    fn fold(&self, prime: &Felt) -> Option<Expr>;

    /// If the op matches one of the var names replaces if with the associated felt value.
    fn replaced_by_const(&self, map: &HashMap<VarStr, Felt>) -> Option<Expr>;

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

    fn is_minus_one(&self, prime: &Felt) -> bool {
        if let Some(n) = self.as_const() {
            return n == (prime.clone() - 1usize);
        }
        false
    }
}

pub trait ConstraintExpr {
    fn is_eq(&self) -> bool;

    fn lhs(&self) -> Expr;

    fn rhs(&self) -> Expr;

    fn is_constant_true(&self) -> bool {
        self.is_eq() && map_consts(&self.lhs(), &self.rhs(), |lhs, rhs| lhs == rhs)
    }

    fn is_constant_false(&self) -> bool {
        self.is_eq() && map_consts(&self.lhs(), &self.rhs(), |lhs, rhs| lhs != rhs)
    }
}

pub trait GetExprHash {
    fn hash(&self) -> ExprHash;
}

pub trait AsExprEq: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_expr_eq(&self) -> &dyn ExprEq;
}

pub trait ExprEq {
    fn expr_eq(&self, other: &dyn ExprLike) -> bool;
}

impl<T: Any + ExprEq> AsExprEq for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_expr_eq(&self) -> &dyn ExprEq {
        self
    }
}

impl<T: Any + PartialEq> ExprEq for T {
    fn expr_eq(&self, other: &dyn ExprLike) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }
}

impl PartialEq<dyn ExprLike> for dyn ExprLike {
    fn eq(&self, other: &dyn ExprLike) -> bool {
        self.expr_eq(other)
    }
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
    + ExprEq
    + AsExprEq
    + GetExprHash
{
}
