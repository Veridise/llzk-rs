use std::ops::Range;

use crate::{
    halo2::Challenge,
    ir::{expr::Felt, CmpOp},
};
use anyhow::{bail, Result};

use super::func::FuncIO;

pub mod lowerable;

pub trait Lowering: ExprLowering {
    fn generate_constraint(
        &self,
        op: CmpOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()>;

    fn num_constraints(&self) -> usize;

    fn checked_generate_constraint(
        &self,
        op: CmpOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()> {
        let before = self.num_constraints();
        self.generate_constraint(op, lhs, rhs)?;
        let after = self.num_constraints();
        if before >= after {
            bail!("Last constraint was not generated!");
        }
        Ok(())
    }

    fn generate_comment(&self, s: String) -> Result<()>;

    fn generate_assume_deterministic(&self, func_io: FuncIO) -> Result<()>;

    fn generate_call(
        &self,
        name: &str,
        selectors: &[Self::CellOutput],
        outputs: &[FuncIO],
    ) -> Result<()>;

    fn generate_assert(&self, expr: &Self::CellOutput) -> Result<()>;
}

pub trait ExprLowering {
    type CellOutput;

    fn lower_sum(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput)
        -> Result<Self::CellOutput>;

    fn lower_product(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput>;

    fn lower_neg(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput>;

    fn lower_challenge(&self, challenge: &Challenge) -> Result<Self::CellOutput>;

    fn lower_constant(&self, f: Felt) -> Result<Self::CellOutput>;

    fn lower_eq(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_lt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_le(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_gt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_ge(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_ne(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_and(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput)
        -> Result<Self::CellOutput>;
    fn lower_or(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_not(&self, value: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_true(&self) -> Result<Self::CellOutput>;
    fn lower_false(&self) -> Result<Self::CellOutput>;

    fn lower_function_input(&self, i: usize) -> FuncIO;
    fn lower_function_output(&self, o: usize) -> FuncIO;

    fn lower_function_inputs(&self, ins: Range<usize>) -> Vec<FuncIO> {
        ins.map(|i| self.lower_function_input(i)).collect()
    }
    fn lower_function_outputs(&self, outs: Range<usize>) -> Vec<FuncIO> {
        outs.map(|o| self.lower_function_output(o)).collect()
    }

    fn lower_funcio<IO>(&self, io: IO) -> Result<Self::CellOutput>
    where
        IO: Into<FuncIO>;
}
