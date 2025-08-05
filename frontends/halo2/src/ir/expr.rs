use anyhow::Result;

use crate::{
    backend::{
        func::FuncIO,
        lowering::{Lowerable, Lowering, LoweringOutput},
    },
    halo2::Field,
};

pub enum IRExpr<F> {
    IO(FuncIO),
    Const(F),
}

impl<F: Field> Lowerable for IRExpr<F> {
    type F = F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        match self {
            IRExpr::IO(func_io) => l.lower_funcio(func_io),
            IRExpr::Const(f) => l.lower_constant(f),
        }
    }
}
