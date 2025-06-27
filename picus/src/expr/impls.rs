use std::fmt;

use crate::{felt::Felt, vars::VarStr};

use super::{
    traits::{ConstantFolding, ExprLike, ExprSize},
    Expr, Wrap,
};

//===----------------------------------------------------------------------===//
// ConstExpr
//===----------------------------------------------------------------------===//

pub struct ConstExpr(Felt);

impl ConstExpr {
    pub fn new(f: Felt) -> Self {
        Self(f)
    }
}

impl ExprSize for ConstExpr {
    fn size(&self) -> usize {
        1
    }
}

impl fmt::Display for ConstExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ConstantFolding for ConstExpr {
    fn as_const(&self) -> Option<Felt> {
        Some(self.0.clone())
    }

    fn fold(&self) -> Option<Expr> {
        None
    }
}

impl ExprLike for ConstExpr {}

//===----------------------------------------------------------------------===//
// VarExpr
//===----------------------------------------------------------------------===//

pub struct VarExpr(VarStr);

impl VarExpr {
    pub fn new(s: VarStr) -> Self {
        Self(s)
    }
}

impl ExprSize for VarExpr {
    fn size(&self) -> usize {
        1
    }
}

impl ConstantFolding for VarExpr {
    fn as_const(&self) -> Option<Felt> {
        None
    }

    fn fold(&self) -> Option<Expr> {
        None
    }
}

impl fmt::Display for VarExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ExprLike for VarExpr {}

//===----------------------------------------------------------------------===//
// BinaryExpr
//===----------------------------------------------------------------------===//

pub trait OpFolder {
    fn fold(&self, lhs: Expr, rhs: Expr) -> Option<Expr>;
}

#[derive(Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl BinaryOp {
    fn fold_add(&self, lhs: Expr, rhs: Expr) -> Option<Expr> {
        if let Some(lhs) = lhs.as_const() {
            if lhs.is_zero() {
                return Some(rhs);
            }
        }
        None
    }

    fn fold_mul(&self, lhs: Expr, rhs: Expr) -> Option<Expr> {
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
    fn fold(&self, lhs: Expr, rhs: Expr) -> Option<Expr> {
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
pub enum ConstraintKind {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

impl OpFolder for ConstraintKind {
    fn fold(&self, _lhs: Expr, _rhs: Expr) -> Option<Expr> {
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

pub struct BinaryExpr<K>(K, Expr, Expr);

impl<K> BinaryExpr<K> {
    pub fn new(k: K, lhs: Expr, rhs: Expr) -> Self {
        Self(k, lhs, rhs)
    }
}

impl<K> BinaryExpr<K> {
    fn lhs(&self) -> Expr {
        self.1.clone()
    }

    fn rhs(&self) -> Expr {
        self.2.clone()
    }

    fn op(&self) -> &K {
        &self.0
    }
}

impl<K: Clone> ExprSize for BinaryExpr<K> {
    fn size(&self) -> usize {
        self.1.size() + self.2.size()
    }
}

impl<K: OpFolder + Clone + fmt::Display + 'static> ConstantFolding for BinaryExpr<K> {
    fn as_const(&self) -> Option<Felt> {
        None
    }

    fn fold(&self) -> Option<Expr> {
        let lhs = self.lhs().fold().unwrap_or_else(|| self.lhs());
        let rhs = self.rhs().fold().unwrap_or_else(|| self.rhs());

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

impl<K: Clone + fmt::Display + OpFolder + 'static> ExprLike for BinaryExpr<K> {}

//===----------------------------------------------------------------------===//
// NegExpr
//===----------------------------------------------------------------------===//

pub struct NegExpr(Expr);

impl NegExpr {
    pub fn new(e: Expr) -> Self {
        Self(e)
    }
}

impl ExprSize for NegExpr {
    fn size(&self) -> usize {
        self.0.size() + 1
    }
}

impl ConstantFolding for NegExpr {
    fn as_const(&self) -> Option<Felt> {
        None
    }

    fn fold(&self) -> Option<Expr> {
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

impl ExprLike for NegExpr {}
