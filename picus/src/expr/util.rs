use crate::felt::Felt;

use super::{traits::ConstraintExpr, Expr};

#[inline]
pub fn map_consts<O: Default>(lhs: &Expr, rhs: &Expr, f: impl Fn(Felt, Felt) -> O) -> O {
    lhs.as_const()
        .zip(rhs.as_const())
        .map(|(lhs, rhs)| f(lhs, rhs))
        .unwrap_or_default()
}

#[inline]
pub fn map_cexpr<O: Default>(
    lhs: &Expr,
    rhs: &Expr,
    f: impl Fn(&dyn ConstraintExpr, &dyn ConstraintExpr) -> O,
) -> O {
    lhs.constraint_expr()
        .zip(rhs.constraint_expr())
        .map(|(lhs, rhs)| f(lhs, rhs))
        .unwrap_or_default()
}
