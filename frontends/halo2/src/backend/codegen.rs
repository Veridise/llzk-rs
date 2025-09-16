use super::lowering::lowerable::LowerableStmt;
use super::lowering::ExprLowering as _;
use super::{func::FuncIO, lowering::Lowering};
use crate::io::{AdviceIO, InstanceIO};
use crate::ir::{IRCtx, ResolvedIRCircuit};
use anyhow::Result;

pub mod lookup;
pub mod strats;

pub trait Codegen<'c: 's, 's>: Sized + 's {
    type FuncOutput: Lowering;
    type Output;
    type State: 'c;

    fn initialize(state: &'s Self::State) -> Self;

    fn define_function(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
    ) -> Result<Self::FuncOutput>;

    fn define_function_with_body<FN, L, I>(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
        f: FN,
    ) -> Result<()>
    where
        FN: FnOnce(&Self::FuncOutput, &[FuncIO], &[FuncIO]) -> Result<I>,
        I: IntoIterator<Item = L>,
        L: LowerableStmt,
    {
        let func = self.define_function(name, inputs, outputs)?;
        let inputs = func.lower_function_inputs(0..inputs);
        let outputs = func.lower_function_outputs(0..outputs);
        let stmts = f(&func, &inputs, &outputs)?;
        for stmt in stmts {
            stmt.lower(&func)?;
        }
        self.on_scope_end(func)
    }

    fn define_main_function(
        &self,
        advice_io: &AdviceIO,
        instance_io: &InstanceIO,
    ) -> Result<Self::FuncOutput>;

    fn define_main_function_with_body<L>(
        &self,
        advice_io: &AdviceIO,
        instance_io: &InstanceIO,
        stmts: impl IntoIterator<Item = L>,
    ) -> Result<()>
    where
        L: LowerableStmt,
    {
        let main = self.define_main_function(advice_io, instance_io)?;
        log::debug!("Defined main function");
        for stmt in stmts {
            stmt.lower(&main)?;
        }
        log::debug!("Lowered function body");
        self.on_scope_end(main)
    }

    fn on_scope_end(&self, _: Self::FuncOutput) -> Result<()> {
        Ok(())
    }

    fn generate_output(self) -> Result<Self::Output>
    where
        Self::Output: 'c;
}

pub trait CodegenStrategy: Default {
    fn codegen<'c: 'st, 's, 'st, C>(
        &self,
        codegen: &C,
        ctx: &IRCtx,
        ir: &ResolvedIRCircuit,
    ) -> Result<()>
    where
        C: Codegen<'c, 'st>;
}
