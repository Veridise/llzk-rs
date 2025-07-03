use crate::vars::VarKind as _;
use display::{TextRepresentable, TextRepresentation};
use impls::{CallStmt, CommentLine, ConstraintStmt};
use std::rc::Rc;
use traits::{
    CallLikeAdaptor, ConstraintLike, ExprArgs, MaybeCallLike, StmtConstantFolding, StmtDisplay,
    StmtLike,
};

use crate::{expr::Expr, vars::VarAllocator};

pub mod display;
mod impls;
pub mod traits;

type Wrap<T> = Rc<T>;

pub type Stmt = Wrap<dyn StmtLike>;

impl StmtDisplay for Stmt {}

impl<S: ExprArgs + ?Sized> ExprArgs for Wrap<S> {
    fn args(&self) -> Vec<Expr> {
        self.as_ref().args()
    }
}

impl<S: ConstraintLike + ?Sized> ConstraintLike for Wrap<S> {
    fn is_constraint(&self) -> bool {
        self.as_ref().is_constraint()
    }
}

impl<S: MaybeCallLike + ?Sized> MaybeCallLike for Wrap<S> {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        self.as_ref().as_call()
    }
}

impl<S: StmtConstantFolding + ?Sized> StmtConstantFolding for Wrap<S> {
    fn fold(&self) -> Option<Stmt> {
        self.as_ref().fold()
    }
}

impl<S: TextRepresentable + ?Sized> TextRepresentable for Wrap<S> {
    fn to_repr(&self) -> TextRepresentation {
        self.as_ref().to_repr()
    }

    fn width_hint(&self) -> usize {
        self.as_ref().width_hint()
    }
}

impl<T> StmtLike for Wrap<T> where T: StmtLike + ?Sized {}

//===----------------------------------------------------------------------===//
// Factories
//===----------------------------------------------------------------------===//

pub fn call<A>(callee: String, inputs: Vec<Expr>, n_outputs: usize, allocator: &A) -> Stmt
where
    A: VarAllocator,
{
    Wrap::new(CallStmt::new(
        callee,
        inputs,
        (0..n_outputs)
            .map(|_| allocator.allocate(A::Kind::temp()))
            .collect(),
    ))
}

pub fn constrain(expr: Expr) -> Stmt {
    Wrap::new(ConstraintStmt::new(expr))
}

pub fn comment(s: String) -> Stmt {
    Wrap::new(CommentLine::new(s))
}
