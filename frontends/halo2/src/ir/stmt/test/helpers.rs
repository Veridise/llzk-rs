use crate::ir::{
    stmt::{constraint::Constraint, IRStmt},
    CmpOp,
};

pub trait ConstraintFactory {
    type Inner;
    type Out;

    fn ctor(&self, op: CmpOp, lhs: Self::Inner, rhs: Self::Inner) -> Self::Out;

    fn eq(&self, lhs: Self::Inner, rhs: Self::Inner) -> Self::Out {
        self.ctor(CmpOp::Eq, lhs, rhs)
    }

    fn lt(&self, lhs: Self::Inner, rhs: Self::Inner) -> Self::Out {
        self.ctor(CmpOp::Lt, lhs, rhs)
    }
}

pub struct IRStmtFactory<T>(std::marker::PhantomData<T>);

impl<T> Default for IRStmtFactory<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> ConstraintFactory for IRStmtFactory<T> {
    type Inner = T;
    type Out = IRStmt<T>;

    fn ctor(&self, op: CmpOp, lhs: Self::Inner, rhs: Self::Inner) -> Self::Out {
        IRStmt::constraint(op, lhs, rhs)
    }
}

pub struct InnerConstraintFactory<T>(std::marker::PhantomData<T>);

impl<T> ConstraintFactory for InnerConstraintFactory<T> {
    type Inner = T;
    type Out = Constraint<T>;

    fn ctor(&self, op: CmpOp, lhs: Self::Inner, rhs: Self::Inner) -> Self::Out {
        Constraint::new(op, lhs, rhs)
    }
}

impl<T> Default for InnerConstraintFactory<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}
