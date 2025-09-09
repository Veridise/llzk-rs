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

mod binary;

pub use binary::{BinaryExpr, BinaryOp, Boolean, ConstraintKind};

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
    fn to_repr(&self) -> TextRepresentation<'_> {
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
    fn to_repr(&self) -> TextRepresentation<'_> {
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
    fn to_repr(&self) -> TextRepresentation<'_> {
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

//===----------------------------------------------------------------------===//
// NotExpr
//===----------------------------------------------------------------------===//

#[derive(Clone, Debug)]
pub struct NotExpr(Expr);

impl NotExpr {
    pub fn new(e: Expr) -> Self {
        Self(e)
    }
}

impl WrappedExpr for NotExpr {
    fn wrap(&self) -> Expr {
        Wrap::new(self.clone())
    }
}

impl ExprSize for NotExpr {
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
            _ => bail!("NotExpr expects 1 argument"),
        }
        .map(|expr| -> Expr { Wrap::new(Self(expr.clone())) }))
    }
}

impl ConstantFolding for NotExpr {
    fn as_const(&self) -> Option<Felt> {
        None
    }

    fn fold(&self, prime: &Felt) -> Option<Expr> {
        self.0
            .fold(prime)
            .map(|inner| -> Expr { Wrap::new(Self(inner)) })
    }
}

impl TextRepresentable for NotExpr {
    fn to_repr(&self) -> TextRepresentation<'_> {
        owned_list!("!", &self.0)
    }

    fn width_hint(&self) -> usize {
        3 + self.0.width_hint()
    }
}

impl MaybeVarLike for NotExpr {
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

impl ConstraintLike for NotExpr {
    fn is_constraint(&self) -> bool {
        false
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        None
    }
}

impl PartialEq for NotExpr {
    fn eq(&self, other: &Self) -> bool {
        self.0 == *other.0
    }
}

impl GetExprHash for NotExpr {
    fn hash(&self) -> ExprHash {
        hash!('!', self.0.hash())
    }
}

impl ExprLike for NotExpr {}
