use crate::expr::Expr;

use super::{
    display::{Display, TextRepresentable},
    Stmt,
};

pub trait ExprArgs {
    fn args(&self) -> Vec<Expr>;
}

pub trait ConstraintLike {
    fn is_constraint(&self) -> bool;
}

pub trait CallLike {
    fn callee(&self) -> &str;

    fn with_new_callee(&self, new_name: String) -> Stmt;
}

pub trait StmtConstantFolding {
    fn fold(&self) -> Option<Stmt>;
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

pub trait StmtDisplay: Clone + AsRef<dyn StmtLike> {
    fn display(&self) -> Display<Self> {
        Display::new(self.clone())
    }
}

pub trait StmtLike:
    ExprArgs + ConstraintLike + MaybeCallLike + StmtConstantFolding + TextRepresentable
{
}
