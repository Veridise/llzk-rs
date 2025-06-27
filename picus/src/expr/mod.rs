use std::{fmt, rc::Rc};

use impls::{BinaryExpr, BinaryOp, ConstExpr, ConstraintKind, NegExpr, OpFolder, VarExpr};
use traits::ExprLike;

use crate::{felt::Felt, vars::VarAllocator};

mod impls;
pub mod traits;

type Wrap<T> = Rc<T>;

/// A pointer to a picus expression.
pub type Expr = Wrap<dyn ExprLike>;

//===----------------------------------------------------------------------===//
// Factories
//===----------------------------------------------------------------------===//

pub fn r#const<I: Into<Felt>>(i: I) -> Expr {
    Wrap::new(ConstExpr::new(i.into()))
}

pub fn var<A, K>(allocator: &A, kind: K) -> Expr
where
    A: VarAllocator,
    K: Into<A::Kind>,
{
    Wrap::new(VarExpr::new(allocator.allocate(kind)))
}

fn binop<K: Clone + fmt::Display + OpFolder + 'static>(kind: K, lhs: &Expr, rhs: &Expr) -> Expr {
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
