use super::lowering::LlzkStructLowering;
use super::state::LlzkCodegenState;
use super::{LlzkOutput, counter::Counter};
use anyhow::{Context as _, Result};

use llzk::prelude::*;
use melior::{
    Context,
    ir::{Location, Module},
};

use crate::backend::llzk::factory::StructIO;
use crate::io::{AdviceIO, InstanceIO};

use crate::backend::codegen::Codegen;

use super::factory;

pub struct LlzkCodegen<'c, 's> {
    state: &'s LlzkCodegenState<'c>,
    module: Module<'c>,
    struct_count: Counter,
}

impl<'c, 's> LlzkCodegen<'c, 's> {
    fn add_struct(&self, s: StructDefOp<'c>) -> Result<StructDefOpRefMut<'c, 's>> {
        let s: StructDefOpRef = self.module.body().append_operation(s.into()).try_into()?;
        Ok(unsafe { StructDefOpRefMut::from_raw(s.to_raw()) })
    }

    fn create_lowering_scope(
        &self,
        name: &str,
        io: StructIO,
        is_main: bool,
    ) -> Result<LlzkStructLowering<'c, 's>> {
        let s =
            factory::create_struct(self.context(), name, self.struct_count.next(), io, is_main)?;
        Ok(LlzkStructLowering::new(self.context(), self.add_struct(s)?))
    }

    fn context(&self) -> &'c Context {
        self.state.context()
    }
}

impl<'c: 's, 's> Codegen<'c, 's> for LlzkCodegen<'c, 's> {
    type FuncOutput = LlzkStructLowering<'c, 's>;
    type Output = LlzkOutput<'c>;
    type State = LlzkCodegenState<'c>;

    fn initialize(state: &'s Self::State) -> Self {
        let module = llzk_module(Location::unknown(state.context()));
        Self {
            state,
            module,
            struct_count: Default::default(),
        }
    }

    fn define_main_function(
        &self,
        advice_io: &AdviceIO,
        instance_io: &InstanceIO,
    ) -> Result<Self::FuncOutput> {
        let name = self.state.params().top_level().unwrap_or("Main");
        log::debug!("Creating Main struct with name '{name}'");
        self.create_lowering_scope(name, StructIO::from_io(advice_io, instance_io), true)
    }

    fn define_function(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
    ) -> Result<Self::FuncOutput> {
        self.create_lowering_scope(name, StructIO::from_io_count(inputs, outputs), false)
    }

    fn on_scope_end(&self, _: Self::FuncOutput) -> Result<()> {
        Ok(())
    }

    fn generate_output(mut self) -> Result<Self::Output> {
        let signal = r#struct::helpers::define_signal_struct(self.context())?;
        self.module.body().insert_operation(0, signal.into());
        verify_operation_with_diags(&self.module.as_operation()).with_context(|| {
            format!(
                "Output module failed verification{}",
                if self.state.optimize() {
                    " (before optimization)"
                } else {
                    ""
                }
            )
        })?;

        if self.state.optimize() {
            let pipeline = create_pipeline(self.context());
            pipeline.run(&mut self.module)?;
        }

        Ok(self.module.into())
    }
}

fn create_pipeline<'c>(context: &'c Context) -> PassManager<'c> {
    let pm = PassManager::new(context);
    pm.nested_under("builtin.module")
        .nested_under("struct.def")
        .add_pass(llzk_passes::create_field_write_validator_pass());
    pm.add_pass(melior_passes::create_canonicalizer());
    pm.add_pass(melior_passes::create_cse());
    pm.add_pass(llzk_passes::create_redundant_read_and_write_elimination_pass());
    pm.nested_under("builtin.module")
        .nested_under("struct.def")
        .add_pass(llzk_passes::create_field_write_validator_pass());

    let opm = pm.as_operation_pass_manager();
    log::debug!("Optimization pipeline: {opm}");
    pm
}

#[cfg(test)]
mod tests {
    use crate::{LlzkParamsBuilder, halo2::Fr};

    use super::*;
    use halo2_proofs::plonk::ConstraintSystem;
    use log::LevelFilter;
    use rstest::{fixture, rstest};
    use simplelog::{Config, TestLogger};

    #[fixture]
    fn common() {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
    }

    #[fixture]
    #[allow(unused_variables)]
    fn ctx(common: ()) -> LlzkContext {
        LlzkContext::new()
    }

    macro_rules! main_function_test {
        ($test_name:ident, $expected:literal, $io:expr $(,)?) => {
            #[rstest]
            fn $test_name(ctx: LlzkContext) {
                let state: LlzkCodegenState =
                    LlzkParamsBuilder::new(&ctx).no_optimize().build().into();
                let codegen = LlzkCodegen::initialize(&state);
                let mut cs = ConstraintSystem::<Fr>::default();
                let (advice_io, instance_io) = ($io)(&mut cs);
                let main = codegen
                    .define_main_function(&advice_io, &instance_io)
                    .unwrap();
                codegen.on_scope_end(main).unwrap();

                let op = codegen.generate_output().unwrap();
                verify_operation_with_diags(&op.module().as_operation()).unwrap();
                mlir_testutils::assert_module_eq_to_file!(op.module(), $expected);
            }
        };
    }

    main_function_test! {
        define_main_function_empty_io,
        "test_files/empty_io.mlir",
        |_| (AdviceIO::empty(), InstanceIO::empty()),
    }

    main_function_test! {
        define_main_function_public_inputs,
        "test_files/public_inputs.mlir",
        |cs: &mut ConstraintSystem<Fr>| (
            AdviceIO::empty(),
            InstanceIO::new(&[(cs.instance_column(), &[0, 1, 2])], &[]).unwrap()
        )
    }

    main_function_test! {
        define_main_function_private_inputs,
        "test_files/private_inputs.mlir",
        |cs: &mut ConstraintSystem<Fr>| (
            AdviceIO::new(&[(cs.advice_column(), &[0, 1, 2])], &[]).unwrap(),
            InstanceIO::empty()
        )
    }

    main_function_test! {
        define_main_function_public_outputs,
        "test_files/public_outputs.mlir",
        |cs: &mut ConstraintSystem<Fr>| (
                AdviceIO::empty(),
                InstanceIO::new(&[], &[(cs.instance_column(), &[0, 1, 2])]).unwrap()
        )
    }

    main_function_test! {
        define_main_function_private_outputs,
        "test_files/private_outputs.mlir",
        |cs: &mut ConstraintSystem<Fr>| (
                AdviceIO::new(&[], &[(cs.advice_column(), &[0, 1, 2])]).unwrap(),
                InstanceIO::empty()
        )
    }

    main_function_test! {
        define_main_function_mixed_io,
        "test_files/mixed_io.mlir",
        |cs: &mut ConstraintSystem<Fr>| {
            let advice_col = cs.advice_column();
            let instance_col = cs.instance_column();
            (
                AdviceIO::new(&[(advice_col, &[0, 1, 2])], &[(advice_col, &[3, 4])]).unwrap(),
                InstanceIO::new(&[(instance_col, &[0, 1])], &[(instance_col, &[2, 3])]).unwrap()
            )
        }
    }
}
