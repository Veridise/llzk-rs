use crate::{
    display::{TextRepresentable, TextRepresentation},
    expr::{
        self,
        impls::{BinaryExpr, ConstExpr, NegExpr},
        traits::ConstantFolding,
        Expr, Wrap,
    },
    felt::Felt,
};

use super::{try_fold, OpFolder, OpLike};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl OpLike for BinaryOp {
    fn extraible(&self) -> bool {
        true
    }
}

impl BinaryOp {
    fn fold_add(&self, lhs: &Expr, rhs: &Expr, _prime: &Felt) -> Option<Expr> {
        if lhs.is_zero() {
            return Some(rhs.clone());
        }

        None
    }

    fn fold_mul(&self, lhs: &Expr, rhs: &Expr, prime: &Felt) -> Option<Expr> {
        if lhs.is_one() {
            return Some(rhs.clone());
        }
        if lhs.is_zero() {
            return Some(lhs.clone());
        }
        if lhs.is_minus_one(prime) {
            return try_fold(NegExpr(rhs.clone()), prime);
        }

        None
    }

    fn fold_sub(&self, lhs: &Expr, rhs: &Expr, prime: &Felt) -> Option<Expr> {
        if lhs.is_zero() && rhs.is_zero() {
            return Some(Wrap::new(ConstExpr(0usize.into())));
        }
        if lhs.is_zero() {
            return try_fold(NegExpr(rhs.clone()), prime);
        }
        if rhs.is_zero() {
            return Some(lhs.clone());
        }
        None
    }

    fn fold_impl(&self, lhs: &Expr, rhs: &Expr, prime: &Felt) -> Option<Expr> {
        match self {
            BinaryOp::Add => self.fold_add(lhs, rhs, prime),
            BinaryOp::Sub => self.fold_sub(lhs, rhs, prime),
            BinaryOp::Mul => self.fold_mul(lhs, rhs, prime),
            BinaryOp::Div => None,
        }
    }
}

impl OpFolder for BinaryOp {
    fn fold(&self, lhs: Expr, rhs: Expr, prime: &Felt) -> Option<Expr> {
        self.fold_impl(&lhs, &rhs, prime).or_else(|| {
            self.flip(&lhs, &rhs)
                .and_then(|e| e.op().fold_impl(&e.lhs(), &e.rhs(), prime))
        })
    }

    fn commutative(&self) -> bool {
        matches!(self, BinaryOp::Add | BinaryOp::Mul)
    }

    fn flip(&self, lhs: &Expr, rhs: &Expr) -> Option<BinaryExpr<Self>> {
        match self {
            BinaryOp::Add => Some(BinaryExpr::new(BinaryOp::Add, rhs.clone(), lhs.clone())),
            BinaryOp::Sub => Some(BinaryExpr::new(BinaryOp::Add, expr::neg(rhs), lhs.clone())),
            BinaryOp::Mul => Some(BinaryExpr::new(BinaryOp::Mul, rhs.clone(), lhs.clone())),
            BinaryOp::Div => None,
        }
    }
}

impl TextRepresentable for BinaryOp {
    fn to_repr(&self) -> TextRepresentation<'_> {
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
