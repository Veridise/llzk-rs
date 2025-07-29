use std::{any::Any, collections::HashSet};

use super::Stmt;
use crate::{
    display::TextRepresentable,
    expr::{traits::ConstraintExpr, Expr},
    felt::Felt,
    vars::VarStr,
};
use anyhow::Result;

pub trait ExprArgs {
    fn args(&self) -> Vec<Expr>;

    fn replace_arg(&mut self, idx: usize, expr: Expr) -> Result<()>;
}

pub trait FreeVars {
    fn free_vars(&self) -> HashSet<&VarStr>;
}

pub trait ConstraintLike {
    fn is_constraint(&self) -> bool;

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr>;
}

pub trait CallLike {
    fn callee(&self) -> &str;

    fn with_new_callee(&self, new_name: String) -> Stmt;
}

pub trait StmtConstantFolding {
    fn fold(&self, prime: &Felt) -> Option<Stmt>;
}

pub trait CallLikeMut: CallLike {
    fn set_callee(&mut self, new_name: String);
}

pub struct CallLikeAdaptor<'a>(&'a dyn CallLike);

impl<'a> CallLikeAdaptor<'a> {
    pub fn new(c: &'a dyn CallLike) -> Self {
        Self(c)
    }
}

impl CallLike for CallLikeAdaptor<'_> {
    fn callee(&self) -> &str {
        self.0.callee()
    }

    fn with_new_callee(&self, new_name: String) -> Stmt {
        self.0.with_new_callee(new_name)
    }
}

pub trait MaybeCallLike {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>>;
}

pub trait AsStmtEq: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_stmt_eq(&self) -> &dyn StmtEq;
}

pub trait StmtEq {
    fn stmt_eq(&self, other: &dyn StmtLike) -> bool;
}

impl<T: Any + StmtEq> AsStmtEq for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_stmt_eq(&self) -> &dyn StmtEq {
        self
    }
}

impl<T: Any + PartialEq> StmtEq for T {
    fn stmt_eq(&self, other: &dyn StmtLike) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }
}

impl PartialEq<dyn StmtLike> for dyn StmtLike {
    fn eq(&self, other: &dyn StmtLike) -> bool {
        self.stmt_eq(other)
    }
}

pub trait StmtLike:
    ExprArgs
    + ConstraintLike
    + MaybeCallLike
    + StmtConstantFolding
    + TextRepresentable
    + std::fmt::Debug
    + StmtEq
    + AsStmtEq
    + FreeVars
{
}
