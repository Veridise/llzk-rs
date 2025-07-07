use std::{collections::HashMap, fmt};

use crate::{
    display::{TextRepresentable, TextRepresentation},
    felt::Felt,
    vars::VarStr,
};

use super::{
    traits::{ConstantFolding, ExprLike, ExprSize, MaybeVarLike, WrappedExpr},
    Expr, Wrap,
};

//===----------------------------------------------------------------------===//
// ConstExpr
//===----------------------------------------------------------------------===//

#[derive(Clone)]
pub struct ConstExpr(Felt);

impl ConstExpr {
    pub fn new(f: Felt) -> Self {
        Self(f)
    }
}

impl WrappedExpr for ConstExpr {
    fn wrap(&self) -> Expr {
        Wrap::new(self.clone())
    }
}

impl ExprSize for ConstExpr {
    fn size(&self) -> usize {
        1
    }

    fn extraible(&self) -> bool {
        false
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

impl TextRepresentable for ConstExpr {
    fn to_repr(&self) -> TextRepresentation {
        self.0.to_repr()
    }

    fn width_hint(&self) -> usize {
        self.0.width_hint()
    }
}

impl MaybeVarLike for ConstExpr {
    fn var_name(&self) -> Option<&VarStr> {
        None
    }

    fn renamed(&self, _: &HashMap<VarStr, VarStr>) -> Option<Expr> {
        None
    }
}

impl ExprLike for ConstExpr {}

//===----------------------------------------------------------------------===//
// VarExpr
//===----------------------------------------------------------------------===//

#[derive(Clone)]
pub struct VarExpr(VarStr);

impl WrappedExpr for VarExpr {
    fn wrap(&self) -> Expr {
        Wrap::new(self.clone())
    }
}

impl VarExpr {
    pub fn new(s: VarStr) -> Self {
        Self(s)
    }
}

impl ExprSize for VarExpr {
    fn size(&self) -> usize {
        1
    }

    fn extraible(&self) -> bool {
        false
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

impl TextRepresentable for VarExpr {
    fn to_repr(&self) -> TextRepresentation {
        self.0.to_repr()
    }

    fn width_hint(&self) -> usize {
        self.0.width_hint()
    }
}

impl MaybeVarLike for VarExpr {
    fn var_name(&self) -> Option<&VarStr> {
        Some(&self.0)
    }

    fn renamed(&self, map: &HashMap<VarStr, VarStr>) -> Option<Expr> {
        if let Some(new_name) = map.get(&self.0).cloned() {
            return Some(Wrap::new(VarExpr(new_name)));
        }
        None
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

impl TextRepresentable for BinaryOp {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::atom(match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
        })
    }

    fn width_hint(&self) -> usize {
        1
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

impl TextRepresentable for ConstraintKind {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::atom(match self {
            ConstraintKind::Lt => "<",
            ConstraintKind::Le => "<=",
            ConstraintKind::Gt => ">",
            ConstraintKind::Ge => ">=",
            ConstraintKind::Eq => "=",
        })
    }

    fn width_hint(&self) -> usize {
        match self {
            ConstraintKind::Lt | ConstraintKind::Gt | ConstraintKind::Eq => 1,
            ConstraintKind::Le | ConstraintKind::Ge => 2,
        }
    }
}

pub trait OpLike: Clone + OpFolder + TextRepresentable + 'static {
    fn extraible(&self) -> bool;
}

impl OpLike for BinaryOp {
    fn extraible(&self) -> bool {
        true
    }
}
impl OpLike for ConstraintKind {
    fn extraible(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct BinaryExpr<K: Clone>(K, Expr, Expr);

impl<K: Clone> BinaryExpr<K> {
    pub fn new(k: K, lhs: Expr, rhs: Expr) -> Self {
        Self(k, lhs, rhs)
    }
}

impl<K: OpLike> WrappedExpr for BinaryExpr<K> {
    fn wrap(&self) -> Expr {
        Wrap::new(self.clone())
    }
}

impl<K: Clone> BinaryExpr<K> {
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

impl<K: OpLike> ExprSize for BinaryExpr<K> {
    fn size(&self) -> usize {
        self.1.size() + self.2.size()
    }

    fn extraible(&self) -> bool {
        self.0.extraible()
    }
}

impl<K: OpLike> ConstantFolding for BinaryExpr<K> {
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

impl<K: OpLike> TextRepresentable for BinaryExpr<K> {
    fn to_repr(&self) -> TextRepresentation {
        owned_list!(self.op(), &self.1, &self.2)
    }

    fn width_hint(&self) -> usize {
        4 + self.0.width_hint() + self.1.width_hint() + self.2.width_hint()
    }
}

impl<K: OpLike> MaybeVarLike for BinaryExpr<K> {
    fn var_name(&self) -> Option<&VarStr> {
        None
    }

    fn renamed(&self, map: &HashMap<VarStr, VarStr>) -> Option<Expr> {
        match (self.lhs().renamed(map), self.rhs().renamed(map)) {
            (None, None) => None,
            (None, Some(rhs)) => Some((self.1.clone(), rhs)),
            (Some(lhs), None) => Some((lhs, self.2.clone())),
            (Some(lhs), Some(rhs)) => Some((lhs, rhs)),
        }
        .map(|(lhs, rhs)| -> Expr { Wrap::new(Self(self.0.clone(), lhs, rhs)) })
    }
}

impl<K: OpLike> ExprLike for BinaryExpr<K> {}

//===----------------------------------------------------------------------===//
// NegExpr
//===----------------------------------------------------------------------===//

#[derive(Clone)]
pub struct NegExpr(Expr);

impl NegExpr {
    pub fn new(e: Expr) -> Self {
        Self(e)
    }
}

impl WrappedExpr for NegExpr {
    fn wrap(&self) -> Expr {
        Wrap::new(self.clone())
    }
}

impl ExprSize for NegExpr {
    fn size(&self) -> usize {
        self.0.size() + 1
    }

    fn extraible(&self) -> bool {
        true
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

impl TextRepresentable for NegExpr {
    fn to_repr(&self) -> TextRepresentation {
        owned_list!("-", &self.0)
    }

    fn width_hint(&self) -> usize {
        3 + self.0.width_hint()
    }
}

impl MaybeVarLike for NegExpr {
    fn var_name(&self) -> Option<&VarStr> {
        None
    }

    fn renamed(&self, map: &HashMap<VarStr, VarStr>) -> Option<Expr> {
        self.0.renamed(map).map(|e| -> Expr { Wrap::new(Self(e)) })
    }
}

impl ExprLike for NegExpr {}
