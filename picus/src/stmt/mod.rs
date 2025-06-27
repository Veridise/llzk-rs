use crate::vars::VarKind as _;
use impls::{CallStmt, ConstraintStmt};
use std::rc::Rc;
use traits::StmtLike;

use crate::{expr::Expr, vars::VarAllocator};

mod impls;
pub mod traits;

type Wrap<T> = Rc<T>;

pub type Stmt = Wrap<dyn StmtLike>;

//===----------------------------------------------------------------------===//
// Factories
//===----------------------------------------------------------------------===//

pub fn call<A>(callee: String, inputs: Vec<Expr>, n_outputs: usize, allocator: &A) -> Stmt
where
    A: VarAllocator,
{
    Wrap::new(CallStmt::new(
        callee,
        inputs,
        (0..n_outputs)
            .map(|_| allocator.allocate(A::Kind::temp()))
            .collect(),
    ))
}

pub fn constrain(expr: Expr) -> Stmt {
    Wrap::new(ConstraintStmt::new(expr))
}
