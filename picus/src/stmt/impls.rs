use std::fmt;

use crate::{
    expr::{
        traits::{ConstantFolding, ExprLike, ExprSize},
        Expr,
    },
    felt::Felt,
    vars::VarStr,
};

use super::{
    traits::{
        CallLike, CallLikeAdaptor, CallLikeAdaptorMut, CallLikeMut, ConstraintLike, ExprArgs,
        MaybeCallLike, StmtConstantFolding, StmtLike,
    },
    Stmt, Wrap,
};

//===----------------------------------------------------------------------===//
// TempVarExpr
//===----------------------------------------------------------------------===//

struct TempVarExpr(VarStr);

impl TempVarExpr {
    pub fn new(s: &VarStr) -> Expr {
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

impl ExprLike for TempVarExpr {}

//===----------------------------------------------------------------------===//
// CallStmt
//===----------------------------------------------------------------------===//

pub struct CallStmt {
    callee: String,
    inputs: Vec<Expr>,
    outputs: Vec<VarStr>,
}

impl CallStmt {
    pub fn new(callee: String, inputs: Vec<Expr>, outputs: Vec<VarStr>) -> Self {
        Self {
            callee,
            inputs,
            outputs,
        }
    }
}

impl ExprArgs for CallStmt {
    fn args(&self) -> Vec<Expr> {
        self.outputs
            .iter()
            .map(TempVarExpr::new)
            .chain(self.inputs.clone().into_iter())
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

impl CallLikeMut for CallStmt {
    fn set_callee(&mut self, new_name: String) {
        self.callee = new_name;
    }
}

impl MaybeCallLike for CallStmt {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        Some(CallLikeAdaptor::new(self))
    }

    fn as_call_mut<'a>(&'a mut self) -> Option<CallLikeAdaptorMut<'a>> {
        Some(CallLikeAdaptorMut::new(self))
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
                .collect(),
            outputs: self.outputs.clone(),
        }))
    }
}

fn print_list<T: fmt::Display>(lst: &[T], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let print = |t: &Option<&T>, f: &mut fmt::Formatter| {
        if let Some(t) = t {
            write!(f, "{t} ")
        } else {
            write!(f, "")
        }
    };
    write!(f, "[")?;
    let mut iter = lst.iter();
    let mut it = iter.next();
    print(&it, f)?;
    while it.is_some() {
        it = iter.next();
        print(&it, f)?;
    }
    write!(f, "]")
}

impl fmt::Display for CallStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(call ")?;
        print_list(&self.outputs, f)?;
        write!(f, " {} ", self.callee)?;
        print_list(&self.inputs, f)?;
        writeln!(f, ")")
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

    fn as_call_mut<'a>(&'a mut self) -> Option<CallLikeAdaptorMut<'a>> {
        None
    }
}

impl fmt::Display for ConstraintStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(assert {})", self.0)
    }
}

impl StmtConstantFolding for ConstraintStmt {
    fn fold(&self) -> Option<Stmt> {
        Some(Wrap::new(Self(self.0.fold().unwrap_or(self.0.clone()))))
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

    fn as_call_mut<'a>(&'a mut self) -> Option<CallLikeAdaptorMut<'a>> {
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

impl StmtLike for CommentLine {}
