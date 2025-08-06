use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash as _, Hasher as _},
};

use anyhow::{bail, Result};
use ops::{OpFolder as _, OpLike};

use crate::{
    display::{TextRepresentable, TextRepresentation},
    expr::{
        traits::{
            ConstantFolding, ConstraintExpr, ExprLike, ExprSize, GetExprHash, MaybeVarLike,
            WrappedExpr,
        },
        util::{map_cexpr, map_consts},
        Expr, ExprHash, Wrap,
    },
    felt::Felt,
    stmt::traits::ConstraintLike,
    vars::VarStr,
};

pub use ops::{arith::BinaryOp, boolean::Boolean, constraint::ConstraintKind};

mod ops;

//===----------------------------------------------------------------------===//
// BinaryExpr
//===----------------------------------------------------------------------===//

#[derive(Clone, Debug)]
pub struct BinaryExpr<K>(K, Expr, Expr)
where
    K: Clone + PartialEq;

impl<K> BinaryExpr<K>
where
    K: Clone + PartialEq,
{
    pub fn new(k: K, lhs: Expr, rhs: Expr) -> Self {
        Self(k, lhs, rhs)
    }
}

macro_rules! binary_expr_common {
    ($K:ty) => {
        impl WrappedExpr for BinaryExpr<$K> {
            fn wrap(&self) -> Expr {
                Wrap::new(self.clone())
            }
        }

        impl ExprSize for BinaryExpr<$K> {
            fn size(&self) -> usize {
                1 + self.1.size() + self.2.size()
            }

            fn extraible(&self) -> bool {
                self.0.extraible()
            }

            fn args(&self) -> Vec<Expr> {
                vec![self.1.clone(), self.2.clone()]
            }

            fn replace_args(&self, args: &[Option<Expr>]) -> Result<Option<Expr>> {
                Ok(match args {
                    [None, None] => None,
                    [Some(lhs), None] => Some((lhs.clone(), self.rhs())),
                    [None, Some(rhs)] => Some((self.lhs(), rhs.clone())),
                    [Some(lhs), Some(rhs)] => Some((lhs.clone(), rhs.clone())),
                    _ => bail!("BinaryExpr expects 2 arguments"),
                }
                .map(|(lhs, rhs)| -> Expr { Wrap::new(Self(self.0.clone(), lhs, rhs)) }))
            }
        }

        impl ConstantFolding for BinaryExpr<$K> {
            fn as_const(&self) -> Option<Felt> {
                None
            }

            fn fold(&self, prime: &Felt) -> Option<Expr> {
                let lhs = self.lhs().fold(prime);
                let rhs = self.rhs().fold(prime);
                match (lhs, rhs) {
                    // If arguments didn't fold then try to fold self and return None if nothing
                    // changed
                    (None, None) => self.op().fold(self.lhs(), self.rhs(), prime),
                    // If at least one of the arguments changed then we need to return a new
                    // version of self to propagate the change downstream
                    (lhs, rhs) => {
                        // Extract or default to the prior value
                        let lhs = lhs.unwrap_or_else(|| self.lhs());
                        let rhs = rhs.unwrap_or_else(|| self.rhs());

                        // Try fold with the new arguments and default to a new version of self with the updated
                        // arguments.
                        self.op()
                            .fold(lhs.clone(), rhs.clone(), prime)
                            .or_else(|| Some(Wrap::new(Self(self.0, lhs, rhs))))
                    }
                }
            }
        }

        impl MaybeVarLike for BinaryExpr<$K> {
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

            fn free_vars(&self) -> HashSet<&VarStr> {
                let mut fv = self.1.free_vars();
                fv.extend(self.2.free_vars());
                fv
            }
        }
    };
}

binary_expr_common!(BinaryOp);
binary_expr_common!(ConstraintKind);
binary_expr_common!(Boolean);

impl<K: Clone + PartialEq> BinaryExpr<K> {
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

impl<K: OpLike> TextRepresentable for BinaryExpr<K> {
    fn to_repr(&self) -> TextRepresentation {
        owned_list!(self.op(), &self.1, &self.2)
    }

    fn width_hint(&self) -> usize {
        4 + self.0.width_hint() + self.1.width_hint() + self.2.width_hint()
    }
}

impl ConstraintExpr for BinaryExpr<ConstraintKind> {
    fn is_eq(&self) -> bool {
        self.0 == ConstraintKind::Eq
    }

    fn lhs(&self) -> Expr {
        self.1.clone()
    }

    fn rhs(&self) -> Expr {
        self.2.clone()
    }

    fn is_constant_true(&self) -> bool {
        map_consts(&self.lhs(), &self.rhs(), |lhs, rhs| {
            self.0.cmp_felts(&lhs, &rhs)
        })
    }

    fn is_constant_false(&self) -> bool {
        map_consts(&self.lhs(), &self.rhs(), |lhs, rhs| {
            !self.0.cmp_felts(&lhs, &rhs)
        })
    }
}

impl ConstraintExpr for BinaryExpr<Boolean> {
    fn is_eq(&self) -> bool {
        false
    }

    fn lhs(&self) -> Expr {
        self.1.clone()
    }

    fn rhs(&self) -> Expr {
        self.2.clone()
    }

    fn is_constant_true(&self) -> bool {
        map_cexpr(&self.1, &self.2, |lhs, rhs| {
            let lhs = lhs.is_constant_true();
            let rhs = rhs.is_constant_true();
            match self.op() {
                Boolean::And => lhs && rhs,
                Boolean::Or => lhs || rhs,
            }
        })
    }

    fn is_constant_false(&self) -> bool {
        map_cexpr(&self.1, &self.2, |lhs, rhs| {
            let lhs = lhs.is_constant_false();
            let rhs = rhs.is_constant_false();
            match self.op() {
                Boolean::And => lhs || rhs,
                Boolean::Or => lhs && rhs,
            }
        })
    }
}

impl ConstraintLike for BinaryExpr<ConstraintKind> {
    fn is_constraint(&self) -> bool {
        true
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        Some(self)
    }
}

impl ConstraintLike for BinaryExpr<BinaryOp> {
    fn is_constraint(&self) -> bool {
        false
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        None
    }
}

impl ConstraintLike for BinaryExpr<Boolean> {
    fn is_constraint(&self) -> bool {
        true
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        Some(self)
    }
}

impl<K: OpLike> BinaryExpr<K> {
    fn eq_flipped(&self, other: &Self, flipped: bool) -> bool {
        if flipped {
            return false;
        }
        self.0
            .flip(&self.1, &self.2)
            .map(|flipped| flipped.eq_impl(other, true))
            .unwrap_or_default()
    }

    fn eq_impl(&self, other: &Self, flipped: bool) -> bool {
        if self.0 == other.0 {
            return (self.1 == *other.1 && self.2 == *other.2)
                || (self.0.commutative() && self.1 == *other.2 && self.2 == *other.1);
        }

        self.eq_flipped(other, flipped)
    }
}

impl<K: OpLike> PartialEq for BinaryExpr<K> {
    fn eq(&self, other: &Self) -> bool {
        self.eq_impl(other, false)
    }
}

impl<K: OpLike> GetExprHash for BinaryExpr<K> {
    fn hash(&self) -> ExprHash {
        hash!(self.0, self.1.hash(), self.2.hash())
    }
}

impl ExprLike for BinaryExpr<ConstraintKind> {}
impl ExprLike for BinaryExpr<BinaryOp> {}
impl ExprLike for BinaryExpr<Boolean> {}
