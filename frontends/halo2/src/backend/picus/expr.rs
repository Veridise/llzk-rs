use super::vars::{VarAllocator, VarKind};
use crate::halo2::Field;
use anyhow::Result;
use std::rc::Rc;

//===----------------------------------------------------------------------===//
// Main traits
//===----------------------------------------------------------------------===//

pub type Wrap<T> = Rc<T>;

pub trait Depth {
    fn depth(&self) -> usize;
}

pub trait PicusExprLike: Depth {}

pub type PicusExpr = Wrap<dyn PicusExprLike>;

//===----------------------------------------------------------------------===//
// Factories
//===----------------------------------------------------------------------===//

pub fn r#const<F: Field>(f: F) -> PicusExpr {
    Wrap::new(ConstExpr(f.clone()))
}

fn var<'a, A, M>(allocator: &'a A, kind: VarKind, meta: M) -> Result<PicusExpr>
where
    A: VarAllocator<'a>,
    M: Into<A::Meta>,
{
    Ok(Wrap::new(VarExpr(
        allocator.allocate(&kind, meta)?.to_owned(),
        kind,
    )))
}

pub fn input_var<'a, A, M>(allocator: &'a A, meta: M) -> Result<PicusExpr>
where
    A: VarAllocator<'a>,
    M: Into<A::Meta>,
{
    var(allocator, VarKind::Input, meta)
}

pub fn output_var<'a, A, M>(allocator: &'a A, meta: M) -> Result<PicusExpr>
where
    A: VarAllocator<'a>,
    M: Into<A::Meta>,
{
    var(allocator, VarKind::Output, meta)
}

pub fn temp_var<'a, A>(allocator: &'a A, meta: A::Meta) -> Result<PicusExpr>
where
    A: VarAllocator<'a>,
{
    var(allocator, VarKind::Temporary, meta)
}

fn binop<K: Clone + 'static>(kind: K, lhs: &PicusExpr, rhs: &PicusExpr) -> PicusExpr {
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

struct ConstExpr<F>(F);

impl<F> Depth for ConstExpr<F> {
    fn depth(&self) -> usize {
        1
    }
}

impl<F> PicusExprLike for ConstExpr<F> {}

//===----------------------------------------------------------------------===//
// VarExpr
//===----------------------------------------------------------------------===//

pub struct VarExpr(String, VarKind);

impl Depth for VarExpr {
    fn depth(&self) -> usize {
        1
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

#[derive(Clone)]
enum ConstraintKind {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

struct BinaryExpr<K>(K, PicusExpr, PicusExpr);

impl<K: Clone> Depth for BinaryExpr<K> {
    fn depth(&self) -> usize {
        self.1.depth() + self.2.depth()
    }
}

impl<K: Clone> PicusExprLike for BinaryExpr<K> {}

//===----------------------------------------------------------------------===//
// NegExpr
//===----------------------------------------------------------------------===//

struct NegExpr(PicusExpr);

impl Depth for NegExpr {
    fn depth(&self) -> usize {
        self.0.depth() + 1
    }
}

impl PicusExprLike for NegExpr {}

//===----------------------------------------------------------------------===//
// CallExpr
//===----------------------------------------------------------------------===//
