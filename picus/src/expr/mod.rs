use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use anyhow::Result;
use impls::{BinaryExpr, BinaryOp, Boolean, ConstExpr, ConstraintKind, NegExpr, VarExpr};
use traits::{
    ConstantFolding, ConstraintExpr, ExprLike, ExprSize, GetExprHash, MaybeVarLike, WrappedExpr,
};

use crate::{
    display::TextRepresentable,
    felt::Felt,
    stmt::traits::ConstraintLike,
    vars::{VarAllocator, VarStr},
};

mod impls;
pub mod traits;
mod util;

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

    fn extraible(&self) -> bool {
        self.as_ref().extraible()
    }

    fn args(&self) -> Vec<Expr> {
        self.as_ref().args()
    }

    fn replace_args(&self, args: &[Option<Expr>]) -> Result<Option<Expr>> {
        self.as_ref().replace_args(args)
    }
}

impl<T: ConstantFolding + ?Sized> ConstantFolding for Wrap<T> {
    fn as_const(&self) -> Option<Felt> {
        self.as_ref().as_const()
    }

    fn fold(&self, prime: &Felt) -> Option<Expr> {
        self.as_ref().fold(prime)
    }
}

impl<T: MaybeVarLike + ?Sized> MaybeVarLike for Wrap<T> {
    fn var_name(&self) -> Option<&VarStr> {
        self.as_ref().var_name()
    }

    fn renamed(&self, map: &HashMap<VarStr, VarStr>) -> Option<Expr> {
        self.as_ref().renamed(map)
    }

    fn free_vars(&self) -> HashSet<&VarStr> {
        self.as_ref().free_vars()
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

impl<T: ConstraintLike + ?Sized> ConstraintLike for Wrap<T> {
    fn is_constraint(&self) -> bool {
        self.as_ref().is_constraint()
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        self.as_ref().constraint_expr()
    }
}

impl<T: GetExprHash + ?Sized> GetExprHash for Wrap<T> {
    fn hash(&self) -> ExprHash {
        self.as_ref().hash()
    }
}

impl<T: ExprLike + PartialEq + 'static> ExprLike for Wrap<T> {}
impl ExprLike for Wrap<dyn ExprLike> {}

impl PartialEq<dyn ExprLike> for Wrap<dyn ExprLike> {
    fn eq(&self, other: &dyn ExprLike) -> bool {
        self.as_ref().expr_eq(other)
    }
}

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct ExprHash(u64);

impl From<u64> for ExprHash {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

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

fn binop(kind: BinaryOp, lhs: &Expr, rhs: &Expr) -> Expr {
    Wrap::new(BinaryExpr::new(kind, lhs.clone(), rhs.clone()))
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

fn constraint(kind: ConstraintKind, lhs: &Expr, rhs: &Expr) -> Expr {
    Wrap::new(BinaryExpr::new(kind, lhs.clone(), rhs.clone()))
}

pub fn lt(lhs: &Expr, rhs: &Expr) -> Expr {
    constraint(ConstraintKind::Lt, lhs, rhs)
}

pub fn le(lhs: &Expr, rhs: &Expr) -> Expr {
    constraint(ConstraintKind::Le, lhs, rhs)
}

pub fn gt(lhs: &Expr, rhs: &Expr) -> Expr {
    constraint(ConstraintKind::Gt, lhs, rhs)
}

pub fn ge(lhs: &Expr, rhs: &Expr) -> Expr {
    constraint(ConstraintKind::Ge, lhs, rhs)
}

pub fn eq(lhs: &Expr, rhs: &Expr) -> Expr {
    constraint(ConstraintKind::Eq, lhs, rhs)
}

pub fn ne(lhs: &Expr, rhs: &Expr) -> Expr {
    constraint(ConstraintKind::Ne, lhs, rhs)
}

pub fn neg(expr: &Expr) -> Expr {
    Wrap::new(NegExpr::new(expr.clone()))
}

fn boolean(kind: Boolean, lhs: &Expr, rhs: &Expr) -> Expr {
    Wrap::new(BinaryExpr::new(kind, lhs.clone(), rhs.clone()))
}

pub fn and(lhs: &Expr, rhs: &Expr) -> Expr {
    boolean(Boolean::And, lhs, rhs)
}

pub fn or(lhs: &Expr, rhs: &Expr) -> Expr {
    boolean(Boolean::Or, lhs, rhs)
}

pub fn r#true() -> Expr {
    eq(&r#const(0), &r#const(0))
}

pub fn r#false() -> Expr {
    ne(&r#const(0), &r#const(0))
}
