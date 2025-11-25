use std::cell::RefCell;
use std::rc::Rc;

use crate::io::{AdviceIO, InstanceIO};
use crate::ir::{IRCtx, ResolvedIRCircuit};
use anyhow::Result;
use haloumi_ir_base::felt::Felt;
use haloumi_ir_base::func::FuncIO;
use haloumi_lowering::{ExprLowering as _, Lowering, lowerable::LowerableStmt};

pub mod strats;

pub trait Codegen<'c: 's, 's>: Sized + 's {
    type FuncOutput: Lowering;
    type Output;
    type State: 'c;

    fn initialize(state: &'s Self::State) -> Self;

    /// Sets the prime field used by the circuit.
    ///
    /// By default does nothing.
    #[allow(unused_variables)]
    fn set_prime_field(&self, prime: Felt) -> Result<()> {
        Ok(())
    }

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
        L: LowerableStmt + std::fmt::Debug,
    {
        let main = self.define_main_function(advice_io, instance_io)?;
        log::debug!("Defined main function");
        for stmt in stmts {
            log::debug!("Lowering statement {stmt:?}");
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

pub trait CodegenStrategy {
    fn codegen<'c: 'st, 's, 'st, C>(
        &self,
        codegen: &C,
        ctx: &IRCtx,
        ir: &ResolvedIRCircuit,
    ) -> Result<()>
    where
        C: Codegen<'c, 'st>;
}

pub trait CodegenParams {
    /// Returns true if inlining is enabled.
    fn inlining_enabled(&self) -> bool;
}

impl<T: CodegenParams> CodegenParams for Rc<RefCell<T>> {
    fn inlining_enabled(&self) -> bool {
        self.borrow().inlining_enabled()
    }
}
