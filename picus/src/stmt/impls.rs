use std::{fmt, slice::Iter};

use crate::{
    expr::{
        traits::{ConstantFolding, ExprLike, ExprSize},
        Expr,
    },
    felt::Felt,
    vars::VarStr,
};

use super::{
    display::{ListPunctuation, TextRepresentable, TextRepresentation},
    traits::{
        CallLike, CallLikeAdaptor, ConstraintLike, ExprArgs, MaybeCallLike, StmtConstantFolding,
        StmtLike,
    },
    Stmt, Wrap,
};

//===----------------------------------------------------------------------===//
// TempVarExpr
//===----------------------------------------------------------------------===//

struct TempVarExpr(VarStr);

impl TempVarExpr {
    pub fn wrapped(s: &VarStr) -> Expr {
        Wrap::new(Self(s.clone()))
    }
}

impl ExprSize for TempVarExpr {
    fn size(&self) -> usize {
        1
    }
}

impl ConstantFolding for TempVarExpr {
    fn as_const(&self) -> Option<Felt> {
        None
    }

    fn fold(&self) -> Option<Expr> {
        None
    }
}

impl fmt::Display for TempVarExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TextRepresentable for TempVarExpr {
    fn to_repr(&self) -> TextRepresentation {
        self.0.to_repr()
    }

    fn width_hint(&self) -> usize {
        self.0.width_hint()
    }
}

impl ExprLike for TempVarExpr {}

//===----------------------------------------------------------------------===//
// CallStmt
//===----------------------------------------------------------------------===//

#[derive(Clone)]
struct Outputs(Vec<VarStr>);
#[derive(Clone)]
struct Inputs(Vec<Expr>);

impl Outputs {
    fn get(&self) -> &[VarStr] {
        &self.0
    }
}

impl From<Vec<VarStr>> for Outputs {
    fn from(value: Vec<VarStr>) -> Self {
        Self(value)
    }
}

impl TextRepresentable for Outputs {
    fn to_repr(&self) -> TextRepresentation {
        self.0.to_repr().with_punct(ListPunctuation::SquareBrackets)
    }

    fn width_hint(&self) -> usize {
        self.0.width_hint()
    }
}

impl Inputs {
    fn iter(&self) -> Iter<Expr> {
        self.0.iter()
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
    fn to_repr(&self) -> TextRepresentation {
        self.0.to_repr().with_punct(ListPunctuation::SquareBrackets)
    }

    fn width_hint(&self) -> usize {
        self.0.width_hint()
    }
}

pub struct CallStmt {
    callee: String,
    inputs: Inputs,
    outputs: Outputs,
}

impl CallStmt {
    pub fn new(callee: String, inputs: Vec<Expr>, outputs: Vec<VarStr>) -> Self {
        Self {
            callee,
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
            .map(TempVarExpr::wrapped)
            .chain(self.inputs.clone())
            .collect()
    }
}

impl ConstraintLike for CallStmt {
    fn is_constraint(&self) -> bool {
        false
    }
}

impl CallLike for CallStmt {
    fn callee(&self) -> &str {
        &self.callee
    }

    fn with_new_callee(&self, callee: String) -> Stmt {
        Wrap::new(Self {
            callee,
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
        })
    }
}

impl MaybeCallLike for CallStmt {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        Some(CallLikeAdaptor::new(self))
    }
}

impl StmtConstantFolding for CallStmt {
    fn fold(&self) -> Option<Stmt> {
        Some(Wrap::new(Self {
            callee: self.callee.clone(),
            inputs: self
                .inputs
                .iter()
                .map(|e| e.fold().unwrap_or(e.clone()))
                .collect::<Vec<_>>()
                .into(),
            outputs: self.outputs.clone(),
        }))
    }
}

impl TextRepresentable for CallStmt {
    fn to_repr(&self) -> TextRepresentation {
        let exprs: Vec<&dyn TextRepresentable> =
            vec![&"call", &self.outputs, &self.callee, &self.inputs];
        TextRepresentation::owned_list(exprs).break_line()
    }

    fn width_hint(&self) -> usize {
        9 + self.callee.len() + self.outputs.width_hint() + self.inputs.width_hint()
    }
}

impl StmtLike for CallStmt {}

//===----------------------------------------------------------------------===//
// ConstraintStmt
//===----------------------------------------------------------------------===//

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
}

impl ConstraintLike for ConstraintStmt {
    fn is_constraint(&self) -> bool {
        true
    }
}

impl MaybeCallLike for ConstraintStmt {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        None
    }
}

impl StmtConstantFolding for ConstraintStmt {
    fn fold(&self) -> Option<Stmt> {
        Some(Wrap::new(Self(self.0.fold().unwrap_or(self.0.clone()))))
    }
}

impl TextRepresentable for ConstraintStmt {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::owned_list(vec![&"assert", self.0.as_ref()]).break_line()
    }

    fn width_hint(&self) -> usize {
        9 + self.0.width_hint()
    }
}

impl StmtLike for ConstraintStmt {}

//===----------------------------------------------------------------------===//
// CommentLine
//===----------------------------------------------------------------------===//

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
}

impl ConstraintLike for CommentLine {
    fn is_constraint(&self) -> bool {
        false
    }
}

impl MaybeCallLike for CommentLine {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        None
    }
}

impl StmtConstantFolding for CommentLine {
    fn fold(&self) -> Option<Stmt> {
        None
    }
}

impl fmt::Display for CommentLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "; {}", self.0)
    }
}

impl TextRepresentable for CommentLine {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::comment(self.0.as_str())
    }

    fn width_hint(&self) -> usize {
        2 + self.0.len()
    }
}

impl StmtLike for CommentLine {}
