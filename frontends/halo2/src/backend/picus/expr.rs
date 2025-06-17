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

pub trait ConstantFolding {
    fn as_const(&self) -> Option<PicusFelt>;

    /// If the expression folded returns Some(expr), otherwise returns None
    fn fold(&self) -> Option<PicusExpr>;

    fn is_one(&self) -> bool {
        if let Some(n) = self.as_const() {
            return n.is_one();
        }
        false
    }

    fn is_zero(&self) -> bool {
        if let Some(n) = self.as_const() {
            return n.is_zero();
        }
        false
    }
}

pub trait PicusExprLike: ExprSize + fmt::Display + ConstantFolding {}

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

fn binop<K: Clone + fmt::Display + OpFolder + 'static>(
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

impl ConstantFolding for ConstExpr {
    fn as_const(&self) -> Option<PicusFelt> {
        Some(self.0.clone())
    }

    fn fold(&self) -> Option<PicusExpr> {
        None
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

impl ConstantFolding for VarExpr {
    fn as_const(&self) -> Option<PicusFelt> {
        None
    }

    fn fold(&self) -> Option<PicusExpr> {
        None
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

trait OpFolder {
    fn fold(&self, lhs: PicusExpr, rhs: PicusExpr) -> Option<PicusExpr>;
}

#[derive(Clone)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl BinaryOp {
    fn fold_add(&self, lhs: PicusExpr, rhs: PicusExpr) -> Option<PicusExpr> {
        if let Some(lhs) = lhs.as_const() {
            if lhs.is_zero() {
                return Some(rhs);
            }
        }
        None
    }

    fn fold_mul(&self, lhs: PicusExpr, rhs: PicusExpr) -> Option<PicusExpr> {
        if let Some(lhs_c) = lhs.as_const() {
            if lhs_c.is_one() {
                return Some(rhs);
            }
            if lhs_c.is_zero() {
                return Some(lhs);
            }
        }
        None
    }
}

impl OpFolder for BinaryOp {
    fn fold(&self, lhs: PicusExpr, rhs: PicusExpr) -> Option<PicusExpr> {
        match self {
            BinaryOp::Add => self
                .fold_add(lhs.clone(), rhs.clone())
                .or_else(|| self.fold_add(rhs, lhs)),
            BinaryOp::Sub => None,
            BinaryOp::Mul => self
                .fold_mul(lhs.clone(), rhs.clone())
                .or_else(|| self.fold_add(rhs, lhs)),
            BinaryOp::Div => None,
        }
    }
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

impl OpFolder for ConstraintKind {
    fn fold(&self, _lhs: PicusExpr, _rhs: PicusExpr) -> Option<PicusExpr> {
        None
    }
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

impl<K> BinaryExpr<K> {
    fn lhs(&self) -> PicusExpr {
        self.1.clone()
    }

    fn rhs(&self) -> PicusExpr {
        self.2.clone()
    }

    fn op(&self) -> &K {
        &self.0
    }
}

impl<K: Clone> ExprSize for BinaryExpr<K> {
    fn depth(&self) -> usize {
        self.1.depth() + self.2.depth()
    }
}

impl<K: OpFolder + Clone + fmt::Display + 'static> ConstantFolding for BinaryExpr<K> {
    fn as_const(&self) -> Option<PicusFelt> {
        None
    }

    fn fold(&self) -> Option<PicusExpr> {
        eprintln!("Op {}, lhs before: {}", self.op(), self.lhs());
        let lhs = self.lhs().fold().unwrap_or_else(|| self.lhs());
        eprintln!("Op {}, lhs after: {}", self.op(), lhs);
        eprintln!("Op {}, rhs before: {}", self.op(), self.rhs());
        let rhs = self.rhs().fold().unwrap_or_else(|| self.rhs());
        eprintln!("Op {}, rhs after: {}", self.op(), rhs);

        self.op()
            .fold(lhs.clone(), rhs.clone())
            .or_else(|| Some(Wrap::new(Self(self.0.clone(), lhs, rhs))))
    }
}

impl<K: fmt::Display> fmt::Display for BinaryExpr<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {} {})", self.0, self.1, self.2)
    }
}

impl<K: Clone + fmt::Display + OpFolder + 'static> PicusExprLike for BinaryExpr<K> {}

//===----------------------------------------------------------------------===//
// NegExpr
//===----------------------------------------------------------------------===//

struct NegExpr(PicusExpr);

impl ExprSize for NegExpr {
    fn depth(&self) -> usize {
        self.0.depth() + 1
    }
}

impl ConstantFolding for NegExpr {
    fn as_const(&self) -> Option<PicusFelt> {
        None
    }

    fn fold(&self) -> Option<PicusExpr> {
        if let Some(e) = self.0.fold() {
            Some(Wrap::new(Self(e)))
        } else {
            None
        }
    }
}

impl fmt::Display for NegExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(- {})", self.0)
    }
}

impl PicusExprLike for NegExpr {}
