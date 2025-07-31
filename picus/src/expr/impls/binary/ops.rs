use crate::{
    display::TextRepresentable,
    expr::{traits::ExprLike, Expr, Wrap},
    felt::Felt,
};

use super::BinaryExpr;

pub mod arith;
pub mod boolean;
pub mod constraint;

pub trait OpFolder: PartialEq + Clone {
    fn fold(&self, lhs: Expr, rhs: Expr, prime: &Felt) -> Option<Expr>;

    fn commutative(&self) -> bool;

    fn flip(&self, lhs: &Expr, rhs: &Expr) -> Option<BinaryExpr<Self>>;
}

pub trait OpLike:
    Clone + PartialEq + OpFolder + TextRepresentable + std::fmt::Debug + std::hash::Hash + 'static
{
    fn extraible(&self) -> bool;
}

/// Tries to fold a newly created expression. If it didn't fold then returns the original
/// expression.
#[inline]
fn try_fold<E: ExprLike>(e: E, prime: &Felt) -> Option<Expr> {
    e.fold(prime).or_else(|| Some(Wrap::new(e)))
}
