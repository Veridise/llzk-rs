use super::{
    expr::{ExprSize, PicusExpr, PicusExprLike},
    vars::{VarAllocator, VarStr},
};
use std::{fmt, rc::Rc};

//===----------------------------------------------------------------------===//
// Main traits
//===----------------------------------------------------------------------===//

pub type Wrap<T> = Rc<T>;

pub trait ExprArgs {
    fn args(&self) -> Vec<PicusExpr>;
}

pub trait ConstraintLike {
    fn is_constraint(&self) -> bool;
}

pub trait CallLike {
    fn callee(&self) -> &str;

    fn with_new_callee(&self, new_name: String) -> PicusStmt;
}

pub trait CallLikeMut: CallLike {
    fn set_callee(&mut self, new_name: String);
}

pub struct CallLikeAdaptor<'a>(&'a dyn CallLike);

pub struct CallLikeAdaptorMut<'a>(&'a mut dyn CallLikeMut);

impl CallLike for CallLikeAdaptor<'_> {
    fn callee(&self) -> &str {
        self.0.callee()
    }

    fn with_new_callee(&self, new_name: String) -> PicusStmt {
        self.0.with_new_callee(new_name)
    }
}

impl CallLike for CallLikeAdaptorMut<'_> {
    fn callee(&self) -> &str {
        self.0.callee()
    }

    fn with_new_callee(&self, new_name: String) -> PicusStmt {
        self.0.with_new_callee(new_name)
    }
}

impl CallLikeMut for CallLikeAdaptorMut<'_> {
    fn set_callee(&mut self, new_name: String) {
        self.0.set_callee(new_name)
    }
}

pub trait MaybeCallLike {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>>;

    fn as_call_mut<'a>(&'a mut self) -> Option<CallLikeAdaptorMut<'a>>;
}

pub trait PicusStmtLike: ExprArgs + ConstraintLike + MaybeCallLike + fmt::Display {}

pub type PicusStmt = Wrap<dyn PicusStmtLike>;

//===----------------------------------------------------------------------===//
// Factories
//===----------------------------------------------------------------------===//

pub fn call<A>(callee: String, inputs: Vec<PicusExpr>, n_outputs: usize, allocator: &A) -> PicusStmt
where
    A: VarAllocator,
{
    Wrap::new(CallStmt {
        callee,
        inputs,
        outputs: (0..n_outputs).map(|_| allocator.allocate_temp()).collect(),
    })
}

pub fn constrain(expr: PicusExpr) -> PicusStmt {
    Wrap::new(PicusConstraint(expr))
}

//===----------------------------------------------------------------------===//
// TempVarExpr
//===----------------------------------------------------------------------===//

pub struct TempVarExpr(VarStr);

impl TempVarExpr {
    pub fn new(s: &VarStr) -> PicusExpr {
        super::expr::Wrap::new(Self(s.clone()))
    }
}

impl ExprSize for TempVarExpr {
    fn depth(&self) -> usize {
        1
    }
}

impl fmt::Display for TempVarExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PicusExprLike for TempVarExpr {}

//===----------------------------------------------------------------------===//
// CallStmt
//===----------------------------------------------------------------------===//

struct CallStmt {
    callee: String,
    inputs: Vec<PicusExpr>,
    outputs: Vec<VarStr>,
}

impl ExprArgs for CallStmt {
    fn args(&self) -> Vec<PicusExpr> {
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

    fn with_new_callee(&self, callee: String) -> PicusStmt {
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
        Some(CallLikeAdaptor(self))
    }

    fn as_call_mut<'a>(&'a mut self) -> Option<CallLikeAdaptorMut<'a>> {
        Some(CallLikeAdaptorMut(self))
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
        write!(f, ")")
    }
}

impl PicusStmtLike for CallStmt {}

//===----------------------------------------------------------------------===//
// ConstraintStmt
//===----------------------------------------------------------------------===//

struct PicusConstraint(PicusExpr);

impl ExprArgs for PicusConstraint {
    fn args(&self) -> Vec<PicusExpr> {
        vec![self.0.clone()]
    }
}

impl ConstraintLike for PicusConstraint {
    fn is_constraint(&self) -> bool {
        true
    }
}

impl MaybeCallLike for PicusConstraint {
    fn as_call<'a>(&'a self) -> Option<CallLikeAdaptor<'a>> {
        None
    }

    fn as_call_mut<'a>(&'a mut self) -> Option<CallLikeAdaptorMut<'a>> {
        None
    }
}

impl fmt::Display for PicusConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(assert {})", self.0)
    }
}

impl PicusStmtLike for PicusConstraint {}
