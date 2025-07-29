use std::{
    collections::{HashMap, HashSet},
    fmt,
    hash::{DefaultHasher, Hash as _, Hasher as _},
};

use anyhow::{anyhow, bail, Result};

use crate::{
    display::{TextRepresentable, TextRepresentation},
    felt::Felt,
    stmt::traits::ConstraintLike,
    vars::VarStr,
};

use super::{
    traits::{
        ConstantFolding, ConstraintExpr, ExprLike, ExprSize, GetExprHash, MaybeVarLike, WrappedExpr,
    },
    Expr, ExprHash, Wrap,
};

macro_rules! hash {
    ($($elt:expr),* $(,)?) => { {
        let mut hasher = DefaultHasher::new();
        '('.hash(&mut hasher);

        $( $elt.hash(&mut hasher); )*

        ')'.hash(&mut hasher);

        hasher.finish().into()
    } };
}

//===----------------------------------------------------------------------===//
// ConstExpr
//===----------------------------------------------------------------------===//

#[derive(Clone, Debug, PartialEq)]
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

    fn args(&self) -> Vec<Expr> {
        vec![]
    }

    fn replace_args(&self, args: &[Option<Expr>]) -> Result<Option<Expr>> {
        if args.is_empty() {
            return Ok(None);
        }
        Err(anyhow!("ConstExpr does not have arguments"))
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

    fn fold(&self, _: &Felt) -> Option<Expr> {
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

    fn free_vars(&self) -> HashSet<&VarStr> {
        Default::default()
    }
}

impl ConstraintLike for ConstExpr {
    fn is_constraint(&self) -> bool {
        false
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        None
    }
}

impl GetExprHash for ConstExpr {
    fn hash(&self) -> ExprHash {
        hash!(self.0)
    }
}

impl ExprLike for ConstExpr {}

//===----------------------------------------------------------------------===//
// VarExpr
//===----------------------------------------------------------------------===//

#[derive(Clone, Debug, PartialEq)]
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

    fn args(&self) -> Vec<Expr> {
        vec![]
    }

    fn replace_args(&self, args: &[Option<Expr>]) -> Result<Option<Expr>> {
        if args.is_empty() {
            return Ok(None);
        }
        Err(anyhow!("VarExpr does not have arguments"))
    }
}

impl ConstantFolding for VarExpr {
    fn as_const(&self) -> Option<Felt> {
        None
    }

    fn fold(&self, _: &Felt) -> Option<Expr> {
        None
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

    fn free_vars(&self) -> HashSet<&VarStr> {
        HashSet::from([&self.0])
    }
}

impl ConstraintLike for VarExpr {
    fn is_constraint(&self) -> bool {
        false
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        None
    }
}

impl GetExprHash for VarExpr {
    fn hash(&self) -> ExprHash {
        hash!(self.0)
    }
}

impl ExprLike for VarExpr {}

//===----------------------------------------------------------------------===//
// BinaryExpr
//===----------------------------------------------------------------------===//

pub trait OpFolder: PartialEq + Clone {
    fn fold(&self, lhs: Expr, rhs: Expr, prime: &Felt) -> Option<Expr>;

    fn commutative(&self) -> bool;

    fn flip(&self, lhs: &Expr, rhs: &Expr) -> Option<BinaryExpr<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Tries to fold a newly created expression. If it didn't fold then returns the original
/// expression.
fn try_fold<E: ExprLike>(e: E, prime: &Felt) -> Option<Expr> {
    e.fold(prime).or_else(|| Some(Wrap::new(e)))
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
            BinaryOp::Sub => Some(BinaryExpr::new(BinaryOp::Add, super::neg(rhs), lhs.clone())),
            BinaryOp::Mul => Some(BinaryExpr::new(BinaryOp::Mul, rhs.clone(), lhs.clone())),
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstraintKind {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

impl OpFolder for ConstraintKind {
    fn fold(&self, _lhs: Expr, _rhs: Expr, _prime: &Felt) -> Option<Expr> {
        None
    }

    fn commutative(&self) -> bool {
        matches!(self, ConstraintKind::Eq)
    }

    fn flip(&self, lhs: &Expr, rhs: &Expr) -> Option<BinaryExpr<Self>> {
        match self {
            ConstraintKind::Lt => Some(BinaryExpr::new(Self::Ge, rhs.clone(), lhs.clone())),
            ConstraintKind::Le => Some(BinaryExpr::new(Self::Gt, rhs.clone(), lhs.clone())),
            ConstraintKind::Gt => Some(BinaryExpr::new(Self::Le, rhs.clone(), lhs.clone())),
            ConstraintKind::Ge => Some(BinaryExpr::new(Self::Lt, rhs.clone(), lhs.clone())),
            ConstraintKind::Eq => Some(BinaryExpr::new(Self::Eq, rhs.clone(), lhs.clone())),
        }
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

pub trait OpLike:
    Clone + PartialEq + OpFolder + TextRepresentable + std::fmt::Debug + std::hash::Hash + 'static
{
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
                    (None, None) => self.op().fold(self.lhs(), self.rhs(), prime),
                    (lhs, rhs) => {
                        let lhs = lhs.unwrap_or_else(|| self.lhs());
                        let rhs = rhs.unwrap_or_else(|| self.rhs());

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

//===----------------------------------------------------------------------===//
// NegExpr
//===----------------------------------------------------------------------===//

#[derive(Clone, Debug)]
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

    fn args(&self) -> Vec<Expr> {
        vec![self.0.clone()]
    }

    fn replace_args(&self, args: &[Option<Expr>]) -> Result<Option<Expr>> {
        Ok(match args {
            [None] => None,
            [Some(expr)] => Some(expr),
            _ => bail!("NegExpr expects 1 argument"),
        }
        .map(|expr| -> Expr { Wrap::new(Self(expr.clone())) }))
    }
}

impl ConstantFolding for NegExpr {
    fn as_const(&self) -> Option<Felt> {
        None
    }

    fn fold(&self, prime: &Felt) -> Option<Expr> {
        let inner = self.0.fold(prime).unwrap_or_else(|| self.0.clone());

        inner
            .as_const()
            .map(|e| {
                let prime = prime.clone();
                assert!(e < prime);
                (prime.clone() - e) % prime
            })
            .map(ConstExpr)
            .map(|e| -> Expr { Wrap::new(e) })
            .or_else(|| -> Option<Expr> { Some(Wrap::new(Self(inner))) })
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

    fn free_vars(&self) -> HashSet<&VarStr> {
        self.0.free_vars()
    }
}

impl ConstraintLike for NegExpr {
    fn is_constraint(&self) -> bool {
        false
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        None
    }
}

impl PartialEq for NegExpr {
    fn eq(&self, other: &Self) -> bool {
        self.0 == *other.0
    }
}

impl GetExprHash for NegExpr {
    fn hash(&self) -> ExprHash {
        hash!('-', self.0.hash())
    }
}

impl ExprLike for NegExpr {}

#[cfg(test)]
mod test_neg_expr {
    use crate::{
        expr::{traits::ConstantFolding as _, Wrap},
        felt::Felt,
    };

    use super::{ConstExpr, NegExpr};

    #[test]
    fn test_const_folding() {
        let prime = Felt::from(7);
        let inner = ConstExpr(Felt::from(2));
        let e = NegExpr(Wrap::new(inner));

        let folded = e.fold(&prime).unwrap();
        let value = folded.as_const().unwrap();
        assert_eq!(value, Felt::from(5));
    }

    #[test]
    fn test_const_folding_0() {
        let prime = Felt::from(7);
        let inner = ConstExpr(Felt::from(0));
        let e = NegExpr(Wrap::new(inner));

        let folded = e.fold(&prime).unwrap();
        let value = folded.as_const().unwrap();
        assert_eq!(value, Felt::from(0));
    }
}
