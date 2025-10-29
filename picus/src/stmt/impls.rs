use std::{collections::HashSet, fmt, slice::Iter};

use anyhow::{Result, anyhow, bail};

use crate::{
    display::{ListPunctuation, TextRepresentable, TextRepresentation},
    expr::{
        Expr, known_var,
        traits::{ConstantFolding, ConstraintExpr, MaybeVarLike},
    },
    felt::Felt,
    ident::Ident,
    vars::VarStr,
};

use super::{
    Stmt, Wrap,
    traits::{
        CallLike, CallLikeAdaptor, ConstraintLike, ExprArgs, FreeVars, MaybeCallLike,
        StmtConstantFolding, StmtLike,
    },
};

//===----------------------------------------------------------------------===//
// CallStmt
//===----------------------------------------------------------------------===//

#[derive(Clone, Debug, PartialEq)]
struct Outputs(Vec<VarStr>);
#[derive(Clone, Debug, PartialEq)]
struct Inputs(Vec<Expr>);

impl Outputs {
    fn get(&self) -> &[VarStr] {
        &self.0
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn replace(&mut self, idx: usize, expr: Expr) -> Result<()> {
        self.0[idx] = expr
            .var_name()
            .ok_or_else(|| anyhow!("Call outputs can only be var expressions"))?
            .clone();
        Ok(())
    }
}

impl From<Vec<VarStr>> for Outputs {
    fn from(value: Vec<VarStr>) -> Self {
        Self(value)
    }
}

impl TextRepresentable for Outputs {
    fn to_repr(&self) -> TextRepresentation<'_> {
        self.0.to_repr().with_punct(ListPunctuation::SquareBrackets)
    }

    fn width_hint(&self) -> usize {
        self.0.width_hint()
    }
}

impl Inputs {
    fn iter(&self) -> Iter<'_, Expr> {
        self.0.iter()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn replace(&mut self, idx: usize, expr: Expr) {
        self.0[idx] = expr;
    }
}

impl IntoIterator for Inputs {
    type Item = <Vec<Expr> as IntoIterator>::Item;

    type IntoIter = <Vec<Expr> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<Expr>> for Inputs {
    fn from(value: Vec<Expr>) -> Self {
        Self(value)
    }
}

impl TextRepresentable for Inputs {
    fn to_repr(&self) -> TextRepresentation<'_> {
        self.0.to_repr().with_punct(ListPunctuation::SquareBrackets)
    }

    fn width_hint(&self) -> usize {
        self.0.width_hint()
    }
}

#[derive(Debug, PartialEq)]
pub struct CallStmt {
    callee: Ident,
    inputs: Inputs,
    outputs: Outputs,
}

impl CallStmt {
    pub fn new(callee: String, inputs: Vec<Expr>, outputs: Vec<VarStr>) -> Self {
        Self {
            callee: callee.into(),
            inputs: inputs.into(),
            outputs: outputs.into(),
        }
    }
}

impl ExprArgs for CallStmt {
    fn args(&self) -> Vec<Expr> {
        self.outputs
            .get()
            .iter()
            .map(known_var)
            .chain(self.inputs.clone())
            .collect()
    }

    fn replace_arg(&mut self, mut idx: usize, expr: Expr) -> Result<()> {
        if idx < self.outputs.len() {
            return self.outputs.replace(idx, expr);
        }
        idx -= self.outputs.len();
        if idx < self.inputs.len() {
            self.inputs.replace(idx, expr);
            return Ok(());
        }
        Err(anyhow!(
            "Idx {idx} is out of bounds for CallStmt (outputs={}, inputs={})",
            self.outputs.len(),
            self.inputs.len()
        ))
    }
}

impl ConstraintLike for CallStmt {
    fn is_constraint(&self) -> bool {
        false
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        None
    }
}

impl CallLike for CallStmt {
    fn callee(&self) -> &str {
        self.callee.value()
    }

    fn with_new_callee(&self, callee: String) -> Stmt {
        Wrap::new(
            Self {
                callee: callee.into(),
                inputs: self.inputs.clone(),
                outputs: self.outputs.clone(),
            }
            .into(),
        )
    }

    fn outputs(&self) -> &[VarStr] {
        &self.outputs.0
    }
}

impl MaybeCallLike for CallStmt {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        Some(CallLikeAdaptor::new(self))
    }
}

impl StmtConstantFolding for CallStmt {
    fn fold(&self, prime: &Felt) -> Option<Stmt> {
        Some(Wrap::new(
            Self {
                callee: self.callee.clone(),
                inputs: self
                    .inputs
                    .iter()
                    .map(|e| e.fold(prime).unwrap_or(e.clone()))
                    .collect::<Vec<_>>()
                    .into(),
                outputs: self.outputs.clone(),
            }
            .into(),
        ))
    }
}

impl TextRepresentable for CallStmt {
    fn to_repr(&self) -> TextRepresentation<'_> {
        owned_list!("call", &self.outputs, &self.callee, &self.inputs).break_line()
    }

    fn width_hint(&self) -> usize {
        9 + self.callee.len() + self.outputs.width_hint() + self.inputs.width_hint()
    }
}

impl FreeVars for CallStmt {
    fn free_vars(&self) -> HashSet<&VarStr> {
        let mut fv = HashSet::from_iter(self.outputs.0.iter());
        fv.extend(self.inputs.iter().flat_map(|e| e.free_vars()));
        fv
    }
}

impl StmtLike for CallStmt {}

//===----------------------------------------------------------------------===//
// ConstraintStmt
//===----------------------------------------------------------------------===//

#[derive(Debug, PartialEq)]
pub struct ConstraintStmt(Expr);

impl ConstraintStmt {
    pub fn new(e: Expr) -> Self {
        Self(e)
    }
}

impl ExprArgs for ConstraintStmt {
    fn args(&self) -> Vec<Expr> {
        vec![self.0.clone()]
    }

    fn replace_arg(&mut self, idx: usize, expr: Expr) -> Result<()> {
        if idx != 0 {
            bail!("Index {idx} is out of bounds for constraint statement");
        }
        self.0 = expr;
        Ok(())
    }
}

impl ConstraintLike for ConstraintStmt {
    fn is_constraint(&self) -> bool {
        self.0.is_constraint()
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        self.0.constraint_expr()
    }
}

impl MaybeCallLike for ConstraintStmt {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        None
    }
}

impl StmtConstantFolding for ConstraintStmt {
    fn fold(&self, prime: &Felt) -> Option<Stmt> {
        Some(Wrap::new(
            Self(self.0.fold(prime).unwrap_or(self.0.clone())).into(),
        ))
    }
}

impl TextRepresentable for ConstraintStmt {
    fn to_repr(&self) -> TextRepresentation<'_> {
        owned_list!("assert", &self.0).break_line()
    }

    fn width_hint(&self) -> usize {
        9 + self.0.width_hint()
    }
}

impl FreeVars for ConstraintStmt {
    fn free_vars(&self) -> HashSet<&VarStr> {
        self.0.free_vars()
    }
}

impl StmtLike for ConstraintStmt {}

//===----------------------------------------------------------------------===//
// CommentLine
//===----------------------------------------------------------------------===//

#[derive(Debug, PartialEq)]
pub struct CommentLine(String);

impl CommentLine {
    pub fn new(s: String) -> Self {
        Self(s)
    }
}

impl ExprArgs for CommentLine {
    fn args(&self) -> Vec<Expr> {
        vec![]
    }

    fn replace_arg(&mut self, _idx: usize, _expr: Expr) -> Result<()> {
        bail!("Comment statement does not have arguments")
    }
}

impl ConstraintLike for CommentLine {
    fn is_constraint(&self) -> bool {
        false
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        None
    }
}

impl MaybeCallLike for CommentLine {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        None
    }
}

impl StmtConstantFolding for CommentLine {
    fn fold(&self, _: &Felt) -> Option<Stmt> {
        None
    }
}

impl fmt::Display for CommentLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "; {}", self.0)
    }
}

impl TextRepresentable for CommentLine {
    fn to_repr(&self) -> TextRepresentation<'_> {
        TextRepresentation::comment(self.0.as_str())
    }

    fn width_hint(&self) -> usize {
        2 + self.0.len()
    }
}

impl FreeVars for CommentLine {
    fn free_vars(&self) -> HashSet<&VarStr> {
        Default::default()
    }
}

impl StmtLike for CommentLine {}

//===----------------------------------------------------------------------===//
// AssumeDeterministicStmt
//===----------------------------------------------------------------------===//

#[derive(PartialEq, Debug)]
pub struct AssumeDeterministicStmt(VarStr);

impl AssumeDeterministicStmt {
    pub fn new(var: &VarStr) -> Self {
        Self(var.clone())
    }
}

impl ExprArgs for AssumeDeterministicStmt {
    fn args(&self) -> Vec<Expr> {
        vec![known_var(&self.0)]
    }

    fn replace_arg(&mut self, idx: usize, expr: Expr) -> Result<()> {
        if idx != 0 {
            bail!("Index {idx} is out of bounds for constraint statement");
        }
        self.0 = expr.var_name().cloned().ok_or_else(|| {
            anyhow!(
                "assume-deterministic statement's argument can only be replaced by variable names"
            )
        })?;
        Ok(())
    }
}

impl ConstraintLike for AssumeDeterministicStmt {
    fn is_constraint(&self) -> bool {
        false
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        None
    }
}

impl MaybeCallLike for AssumeDeterministicStmt {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        None
    }
}

impl StmtConstantFolding for AssumeDeterministicStmt {
    fn fold(&self, _prime: &Felt) -> Option<Stmt> {
        None
    }
}

impl TextRepresentable for AssumeDeterministicStmt {
    fn to_repr(&self) -> TextRepresentation<'_> {
        owned_list!("assume-deterministic", &self.0).break_line()
    }

    fn width_hint(&self) -> usize {
        self.0.width_hint() + "(assume-deterministic)".len()
    }
}

impl FreeVars for AssumeDeterministicStmt {
    fn free_vars(&self) -> HashSet<&VarStr> {
        [&self.0].into()
    }
}

impl StmtLike for AssumeDeterministicStmt {}

//===----------------------------------------------------------------------===//
// PostConditionStmt
//===----------------------------------------------------------------------===//

#[derive(Debug, PartialEq)]
pub struct PostConditionStmt(Expr);

impl PostConditionStmt {
    pub fn new(e: Expr) -> Self {
        Self(e)
    }
}

impl ExprArgs for PostConditionStmt {
    fn args(&self) -> Vec<Expr> {
        vec![self.0.clone()]
    }

    fn replace_arg(&mut self, idx: usize, expr: Expr) -> Result<()> {
        if idx != 0 {
            bail!("Index {idx} is out of bounds for post-condition statement");
        }
        self.0 = expr;
        Ok(())
    }
}

impl ConstraintLike for PostConditionStmt {
    fn is_constraint(&self) -> bool {
        false
    }

    fn constraint_expr(&self) -> Option<&dyn ConstraintExpr> {
        None
    }
}

impl MaybeCallLike for PostConditionStmt {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        None
    }
}

impl StmtConstantFolding for PostConditionStmt {
    fn fold(&self, prime: &Felt) -> Option<Stmt> {
        Some(Wrap::new(
            Self(self.0.fold(prime).unwrap_or(self.0.clone())).into(),
        ))
    }
}

const POST_CONDITION_KW: &'static str = "post-condition";

impl TextRepresentable for PostConditionStmt {
    fn to_repr(&self) -> TextRepresentation<'_> {
        owned_list!(POST_CONDITION_KW, &self.0).break_line()
    }

    fn width_hint(&self) -> usize {
        3 + POST_CONDITION_KW.len() + self.0.width_hint()
    }
}

impl FreeVars for PostConditionStmt {
    fn free_vars(&self) -> HashSet<&VarStr> {
        self.0.free_vars()
    }
}

impl StmtLike for PostConditionStmt {}
