use super::{
    output::PicusFelt,
    vars::{VarAllocator, VarStr},
};
use crate::halo2::Field;
use std::{fmt, rc::Rc};

//===----------------------------------------------------------------------===//
// Main traits
//===----------------------------------------------------------------------===//

pub type Wrap<T> = Rc<T>;

pub trait ExprSize {
    fn depth(&self) -> usize;
}

pub trait PicusExprLike: ExprSize + fmt::Display {}

pub type PicusExpr = Wrap<dyn PicusExprLike>;

//===----------------------------------------------------------------------===//
// Factories
//===----------------------------------------------------------------------===//

pub fn r#const<F: Field>(f: F) -> PicusExpr {
    Wrap::new(ConstExpr(f.into()))
}

pub fn var<A, K>(allocator: &A, kind: K) -> PicusExpr
where
    A: VarAllocator,
    K: Into<A::Kind>,
{
    Wrap::new(VarExpr(allocator.allocate(kind)))
}

fn binop<K: Clone + fmt::Display + 'static>(
    kind: K,
    lhs: &PicusExpr,
    rhs: &PicusExpr,
) -> PicusExpr {
    Wrap::new(BinaryExpr(kind.clone(), rhs.clone(), lhs.clone()))
}

pub fn add(lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
    binop(BinaryOp::Add, lhs, rhs)
}

pub fn sub(lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
    binop(BinaryOp::Sub, lhs, rhs)
}

pub fn mul(lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
    binop(BinaryOp::Mul, lhs, rhs)
}

pub fn div(lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
    binop(BinaryOp::Div, lhs, rhs)
}

pub fn lt(lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
    binop(ConstraintKind::Lt, lhs, rhs)
}

pub fn le(lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
    binop(ConstraintKind::Le, lhs, rhs)
}

pub fn gt(lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
    binop(ConstraintKind::Gt, lhs, rhs)
}

pub fn ge(lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
    binop(ConstraintKind::Ge, lhs, rhs)
}

pub fn eq(lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
    binop(ConstraintKind::Eq, lhs, rhs)
}

pub fn neg(expr: &PicusExpr) -> PicusExpr {
    Wrap::new(NegExpr(expr.clone()))
}

//===----------------------------------------------------------------------===//
// ConstExpr
//===----------------------------------------------------------------------===//

struct ConstExpr(PicusFelt);

impl ExprSize for ConstExpr {
    fn depth(&self) -> usize {
        1
    }
}

impl fmt::Display for ConstExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PicusExprLike for ConstExpr {}

//===----------------------------------------------------------------------===//
// VarExpr
//===----------------------------------------------------------------------===//

pub struct VarExpr(VarStr);

impl ExprSize for VarExpr {
    fn depth(&self) -> usize {
        1
    }
}

impl fmt::Display for VarExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PicusExprLike for VarExpr {}

//===----------------------------------------------------------------------===//
// BinaryExpr
//===----------------------------------------------------------------------===//

#[derive(Clone)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
            }
        )
    }
}

#[derive(Clone)]
enum ConstraintKind {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

impl fmt::Display for ConstraintKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ConstraintKind::Lt => "<",
                ConstraintKind::Le => "<=",
                ConstraintKind::Gt => ">",
                ConstraintKind::Ge => ">=",
                ConstraintKind::Eq => "=",
            }
        )
    }
}

struct BinaryExpr<K>(K, PicusExpr, PicusExpr);

impl<K: Clone> ExprSize for BinaryExpr<K> {
    fn depth(&self) -> usize {
        self.1.depth() + self.2.depth()
    }
}

impl<K: fmt::Display> fmt::Display for BinaryExpr<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {} {})", self.0, self.1, self.2)
    }
}

impl<K: Clone + fmt::Display> PicusExprLike for BinaryExpr<K> {}

//===----------------------------------------------------------------------===//
// NegExpr
//===----------------------------------------------------------------------===//

struct NegExpr(PicusExpr);

impl ExprSize for NegExpr {
    fn depth(&self) -> usize {
        self.0.depth() + 1
    }
}

impl fmt::Display for NegExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(- {})", self.0)
    }
}

impl PicusExprLike for NegExpr {}
