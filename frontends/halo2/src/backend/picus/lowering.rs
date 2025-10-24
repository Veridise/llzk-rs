use super::vars::{NamingConvention, VarKey, VarKeySeed};
use crate::ir::expr::Felt;
use crate::{
    backend::{
        func::{ArgNo, FieldId, FuncIO},
        lowering::{ExprLowering, Lowering},
    },
    halo2::Challenge,
    ir::CmpOp,
};
use anyhow::Result;
use picus::{ModuleLike as _, expr, stmt};

pub type PicusModuleRef = picus::ModuleRef<VarKey>;
pub(super) type PicusExpr = picus::expr::Expr;

#[derive(Clone, Debug)]
pub struct PicusModuleLowering {
    module: PicusModuleRef,
    naming_convention: NamingConvention,
}

impl PicusModuleLowering {
    pub fn new(module: PicusModuleRef, naming_convention: NamingConvention) -> Self {
        Self {
            module,
            naming_convention,
        }
    }
}

impl PicusModuleLowering {
    pub fn lower_func_io(&self, func_io: FuncIO) -> PicusExpr {
        let seed = VarKeySeed::io(func_io, self.naming_convention);
        expr::var(&self.module, seed)
    }
}

impl Lowering for PicusModuleLowering {
    fn generate_constraint(
        &self,
        op: CmpOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()> {
        self.module.borrow_mut().add_constraint(match op {
            CmpOp::Eq => expr::eq(lhs, rhs),
            CmpOp::Lt => expr::lt(lhs, rhs),
            CmpOp::Le => expr::le(lhs, rhs),
            CmpOp::Gt => expr::gt(lhs, rhs),
            CmpOp::Ge => expr::ge(lhs, rhs),
            CmpOp::Ne => unimplemented!(),
        });
        Ok(())
    }

    fn num_constraints(&self) -> usize {
        self.module.constraints_len()
    }

    fn generate_comment(&self, s: String) -> Result<()> {
        self.module.borrow_mut().add_stmt(stmt::comment(s));
        Ok(())
    }

    fn generate_call(
        &self,
        name: &str,
        inputs: &[Self::CellOutput],
        outputs: &[FuncIO],
    ) -> Result<()> {
        let stmt = stmt::call(
            name.to_owned(),
            inputs.to_vec(),
            outputs
                .iter()
                .copied()
                .map(|o| self.lower_func_io(o))
                .collect(),
        )?;
        self.module.borrow_mut().add_stmt(stmt);
        Ok(())
    }

    fn generate_assume_deterministic(&self, func_io: FuncIO) -> Result<()> {
        let stmt = stmt::assume_deterministic(self.lower_func_io(func_io))?;
        self.module.borrow_mut().add_stmt(stmt);
        Ok(())
    }

    fn generate_assert(&self, expr: &Self::CellOutput) -> Result<()> {
        let stmt = stmt::constrain(expr.clone());
        self.module.borrow_mut().add_stmt(stmt);
        Ok(())
    }

    fn generate_post_condition(&self, expr: &Self::CellOutput) -> Result<()> {
        let stmt = stmt::post_condition(expr.clone());
        self.module.borrow_mut().add_stmt(stmt);
        Ok(())
    }
}

impl ExprLowering for PicusModuleLowering {
    type CellOutput = PicusExpr;

    fn lower_sum(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        Ok(expr::add(lhs, rhs))
    }

    fn lower_product(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        Ok(expr::mul(lhs, rhs))
    }

    fn lower_neg(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::neg(expr))
    }

    fn lower_challenge(&self, _challenge: &Challenge) -> Result<Self::CellOutput> {
        unimplemented!()
    }

    fn lower_constant(&self, f: Felt) -> Result<Self::CellOutput> {
        let expr = expr::r#const(f);
        log::debug!(
            "[PicusBackend::lower_constant] Constant value {f:?} becomes expression {expr:?}"
        );
        Ok(expr)
    }

    fn lower_eq(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::eq(lhs, rhs))
    }

    fn lower_and(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        Ok(expr::and(lhs, rhs))
    }

    fn lower_or(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::or(lhs, rhs))
    }

    fn lower_function_input(&self, i: usize) -> FuncIO {
        ArgNo::from(i).into()
    }

    fn lower_function_output(&self, o: usize) -> FuncIO {
        FieldId::from(o).into()
    }

    fn lower_funcio<IO>(&self, io: IO) -> Result<Self::CellOutput>
    where
        IO: Into<FuncIO>,
    {
        Ok(self.lower_func_io(io.into()))
    }

    fn lower_lt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::lt(lhs, rhs))
    }

    fn lower_le(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::le(lhs, rhs))
    }

    fn lower_gt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::gt(lhs, rhs))
    }

    fn lower_ge(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::ge(lhs, rhs))
    }

    fn lower_ne(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::ne(lhs, rhs))
    }

    fn lower_not(&self, value: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::not(value))
    }

    fn lower_true(&self) -> Result<Self::CellOutput> {
        Ok(expr::eq(&expr::r#const(0), &expr::r#const(0)))
    }

    fn lower_false(&self) -> Result<Self::CellOutput> {
        Ok(expr::eq(&expr::r#const(0), &expr::r#const(1)))
    }

    fn lower_det(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::det(expr))
    }

    fn lower_implies(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        Ok(expr::implies(lhs, rhs))
    }

    fn lower_iff(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        Ok(expr::iff(lhs, rhs))
    }
}
