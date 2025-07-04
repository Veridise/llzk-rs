use std::{fmt, rc::Rc};

use impls::{BinaryExpr, BinaryOp, ConstExpr, ConstraintKind, NegExpr, OpFolder, OpLike, VarExpr};
use traits::{ConstantFolding, ExprLike, ExprSize, MaybeVarLike, WrappedExpr};

use crate::{
    display::TextRepresentable,
    felt::Felt,
    vars::{VarAllocator, VarStr},
};

mod impls;
pub mod traits;

type Wrap<T> = Rc<T>;

/// A pointer to a picus expression.
pub type Expr = Wrap<dyn ExprLike>;

impl WrappedExpr for Wrap<dyn ExprLike> {
    fn wrap(&self) -> Expr {
        self.as_ref().wrap()
    }
}

impl<T: ExprLike + 'static> WrappedExpr for Wrap<T> {
    fn wrap(&self) -> Expr {
        self.clone()
    }
}

impl<T: ExprLike + 'static + ?Sized> ExprSize for Wrap<T> {
    fn size(&self) -> usize {
        self.as_ref().size()
    }
}

impl<T: ConstantFolding + ?Sized> ConstantFolding for Wrap<T> {
    fn as_const(&self) -> Option<Felt> {
        self.as_ref().as_const()
    }

    fn fold(&self) -> Option<Expr> {
        self.as_ref().fold()
    }
}

impl<T: MaybeVarLike + ?Sized> MaybeVarLike for Wrap<T> {
    fn var_name(&self) -> Option<&VarStr> {
        self.as_ref().var_name()
    }
}

impl<T: TextRepresentable + ?Sized> TextRepresentable for Wrap<T> {
    fn to_repr(&self) -> crate::display::TextRepresentation {
        self.as_ref().to_repr()
    }

    fn width_hint(&self) -> usize {
        self.as_ref().width_hint()
    }
}

impl<T: ExprLike + 'static> ExprLike for Wrap<T> {}
impl ExprLike for Wrap<dyn ExprLike> {}

//===----------------------------------------------------------------------===//
// Factories
//===----------------------------------------------------------------------===//

pub fn r#const<I: Into<Felt>>(i: I) -> Expr {
    Wrap::new(ConstExpr::new(i.into()))
}

pub fn var<A, K>(allocator: &A, kind: K) -> Expr
where
    A: VarAllocator,
    K: Into<A::Kind> + Into<VarStr> + Clone,
{
    Wrap::new(VarExpr::new(allocator.allocate(kind)))
}

pub(crate) fn known_var(var: &VarStr) -> Expr {
    Wrap::new(VarExpr::new(var.clone()))
}

fn binop<K: OpLike>(kind: K, lhs: &Expr, rhs: &Expr) -> Expr {
    Wrap::new(BinaryExpr::new(kind.clone(), rhs.clone(), lhs.clone()))
}

pub fn add(lhs: &Expr, rhs: &Expr) -> Expr {
    binop(BinaryOp::Add, lhs, rhs)
}

pub fn sub(lhs: &Expr, rhs: &Expr) -> Expr {
    binop(BinaryOp::Sub, lhs, rhs)
}

pub fn mul(lhs: &Expr, rhs: &Expr) -> Expr {
    binop(BinaryOp::Mul, lhs, rhs)
}

pub fn div(lhs: &Expr, rhs: &Expr) -> Expr {
    binop(BinaryOp::Div, lhs, rhs)
}

pub fn lt(lhs: &Expr, rhs: &Expr) -> Expr {
    binop(ConstraintKind::Lt, lhs, rhs)
}

pub fn le(lhs: &Expr, rhs: &Expr) -> Expr {
    binop(ConstraintKind::Le, lhs, rhs)
}

pub fn gt(lhs: &Expr, rhs: &Expr) -> Expr {
    binop(ConstraintKind::Gt, lhs, rhs)
}

pub fn ge(lhs: &Expr, rhs: &Expr) -> Expr {
    binop(ConstraintKind::Ge, lhs, rhs)
}

pub fn eq(lhs: &Expr, rhs: &Expr) -> Expr {
    binop(ConstraintKind::Eq, lhs, rhs)
}

pub fn neg(expr: &Expr) -> Expr {
    Wrap::new(NegExpr::new(expr.clone()))
}
