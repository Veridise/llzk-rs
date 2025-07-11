use crate::display::{TextRepresentable, TextRepresentation};
use crate::expr::traits::ConstraintExpr;
use crate::vars::{Temp, VarKind as _};
use anyhow::Result;
use impls::{CallStmt, CommentLine, ConstraintStmt};
use std::{cell::RefCell, ops::Deref as _, rc::Rc};
use traits::{
    CallLikeAdaptor, ConstraintLike, ExprArgs, MaybeCallLike, StmtConstantFolding, StmtLike,
};

use crate::{expr::Expr, vars::VarAllocator};

mod impls;
pub mod traits;

type Wrap<T> = Rc<RefCell<T>>;

pub type Stmt = Wrap<dyn StmtLike>;

impl<S: ExprArgs + ?Sized> ExprArgs for Wrap<S> {
    fn args(&self) -> Vec<Expr> {
        unsafe { (*self.as_ptr()).args() }
    }

    fn replace_arg(&mut self, idx: usize, expr: Expr) -> Result<()> {
        self.borrow_mut().replace_arg(idx, expr)
    }
}

impl<S: ConstraintLike + ?Sized> ConstraintLike for Wrap<S> {
    fn is_constraint(&self) -> bool {
        self.borrow().is_constraint()
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        unsafe { (*self.as_ptr()).constraint_expr() }
    }
}

impl<S: MaybeCallLike + ?Sized> MaybeCallLike for Wrap<S> {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        unsafe { (*self.as_ptr()).as_call() }
    }
}

impl<S: StmtConstantFolding + ?Sized> StmtConstantFolding for Wrap<S> {
    fn fold(&self) -> Option<Stmt> {
        self.borrow().fold()
    }
}

impl<S: TextRepresentable + ?Sized> TextRepresentable for Wrap<S> {
    fn to_repr(&self) -> TextRepresentation {
        unsafe { (*self.as_ptr()).to_repr() }
    }

    fn width_hint(&self) -> usize {
        self.borrow().width_hint()
    }
}

impl<T> StmtLike for Wrap<T> where T: StmtLike + ?Sized {}

//===----------------------------------------------------------------------===//
// Factories
//===----------------------------------------------------------------------===//

pub fn call<A>(
    callee: String,
    inputs: Vec<Expr>,
    n_outputs: usize,
    allocator: &A,
    ctx: <A::Kind as Temp>::Ctx,
) -> Stmt
where
    A: VarAllocator,
    A::Kind: Temp,
{
    Wrap::new(
        CallStmt::new(
            callee,
            inputs,
            (0..n_outputs)
                .map(|_| allocator.allocate(A::Kind::temp(ctx)))
                .collect(),
        )
        .into(),
    )
}

pub fn constrain(expr: Expr) -> Stmt {
    Wrap::new(ConstraintStmt::new(expr).into())
}

pub fn comment(s: String) -> Stmt {
    Wrap::new(CommentLine::new(s).into())
}
