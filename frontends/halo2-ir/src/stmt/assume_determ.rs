use haloumi_ir_base::{
    equivalency::{EqvRelation, SymbolicEqv},
    func::FuncIO,
};
use haloumi_lowering::{Lowering, lowerable::LowerableStmt};

pub struct AssumeDeterministic(FuncIO);

impl AssumeDeterministic {
    pub fn new(f: FuncIO) -> Self {
        Self(f)
    }

    pub fn value(&self) -> FuncIO {
        self.0
    }

    pub fn value_mut(&mut self) -> &mut FuncIO {
        &mut self.0
    }
}

impl LowerableStmt for AssumeDeterministic {
    fn lower<L>(self, l: &L) -> haloumi_lowering::Result<()>
    where
        L: Lowering + ?Sized,
    {
        l.generate_assume_deterministic(self.0)
    }
}

impl Clone for AssumeDeterministic {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl PartialEq for AssumeDeterministic {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::fmt::Debug for AssumeDeterministic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "assume-deterministic {:?}", self.0)
    }
}

impl EqvRelation<AssumeDeterministic, AssumeDeterministic> for SymbolicEqv
where
    SymbolicEqv: EqvRelation<FuncIO, FuncIO>,
{
    fn equivalent(lhs: &AssumeDeterministic, rhs: &AssumeDeterministic) -> bool {
        SymbolicEqv::equivalent(&lhs.0, &rhs.0)
    }
}
