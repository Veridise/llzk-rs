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
use crate::ir::expr::Felt;

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
        let struct_name = self.state.params().top_level().unwrap_or("Main");
        log::debug!("Creating struct with name '{struct_name}'");
        let s = factory::create_struct(
            self.context(),
            struct_name,
            self.struct_count.next(),
            StructIO::new_from_io(advice_io, instance_io),
            true,
        )?;
        log::debug!("Created struct object {s:?}");
        Ok(LlzkStructLowering::new(self.context(), self.add_struct(s)?))
    }

    fn define_function(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
    ) -> Result<Self::FuncOutput> {
        let s = factory::create_struct(
            self.context(),
            name,
            self.struct_count.next(),
            StructIO::new_from_io_count(inputs, outputs),
            false,
        )?;
        Ok(LlzkStructLowering::new(self.context(), self.add_struct(s)?))
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
    use crate::{
        LlzkParamsBuilder,
        halo2::{ConstraintSystem, Fr},
    };

    use super::*;
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
        let context = LlzkContext::new();
        context
    }

    #[rstest]
    fn define_main_function_empty_io(ctx: LlzkContext) {
        let state: LlzkCodegenState = LlzkParamsBuilder::new(&ctx).no_optimize().build().into();
        let codegen = LlzkCodegen::initialize(&state);
        let advice_io = AdviceIO::empty();
        let instance_io = InstanceIO::empty();
        let main = codegen
            .define_main_function(&advice_io, &instance_io)
            .unwrap();
        codegen.on_scope_end(main).unwrap();

        let op = codegen.generate_output().unwrap();
        assert!(
            op.module().as_operation().verify(),
            "Top level module failed verification"
        );
        let op_str = format!("{}", op);
        similar_asserts::assert_eq!(
            op_str,
            r#"module attributes {veridise.lang = "llzk"} {
  struct.def @Signal<[]> {
    struct.field @reg : !felt.type {llzk.pub}
    function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Signal<[]>>
      struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
      function.return %self : !struct.type<@Signal<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint} {
      function.return
    }
  }
  struct.def @Main<[]> {
    function.def @compute() -> !struct.type<@Main<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Main<[]>>
      function.return %self : !struct.type<@Main<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Main<[]>>) attributes {function.allow_constraint} {
      function.return
    }
  }
}
"#
        );
    }

    #[rstest]
    fn define_main_function_public_inputs(ctx: LlzkContext) {
        let state: LlzkCodegenState = LlzkParamsBuilder::new(&ctx).no_optimize().build().into();
        let codegen = LlzkCodegen::initialize(&state);
        let mut cs = ConstraintSystem::<Fr>::default();
        let instance_col = cs.instance_column();
        let advice_io = AdviceIO::empty();
        let instance_io = InstanceIO::new(&[(instance_col, &[0, 1, 2])], &[]);
        let main = codegen
            .define_main_function(&advice_io, &instance_io)
            .unwrap();
        codegen.on_scope_end(main).unwrap();

        let op = codegen.generate_output().unwrap();
        assert!(
            op.module().as_operation().verify(),
            "Top level module failed verification"
        );
        let op_str = format!("{}", op);
        similar_asserts::assert_eq!(
            op_str,
            r#"module attributes {veridise.lang = "llzk"} {
  struct.def @Signal<[]> {
    struct.field @reg : !felt.type {llzk.pub}
    function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Signal<[]>>
      struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
      function.return %self : !struct.type<@Signal<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint} {
      function.return
    }
  }
  struct.def @Main<[]> {
    function.def @compute(%arg0: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg1: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg2: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}) -> !struct.type<@Main<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Main<[]>>
      function.return %self : !struct.type<@Main<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Main<[]>>, %arg1: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg2: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg3: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}) attributes {function.allow_constraint} {
      function.return
    }
  }
}
"#
        );
    }

    #[rstest]
    fn define_main_function_private_inputs(ctx: LlzkContext) {
        let state: LlzkCodegenState = LlzkParamsBuilder::new(&ctx).no_optimize().build().into();
        let codegen = LlzkCodegen::initialize(&state);
        let mut cs = ConstraintSystem::<Fr>::default();
        let advice_col = cs.advice_column();
        let advice_io = AdviceIO::new(&[(advice_col, &[0, 1, 2])], &[]);
        let instance_io = InstanceIO::new(&[], &[]);
        let main = codegen
            .define_main_function(&advice_io, &instance_io)
            .unwrap();
        codegen.on_scope_end(main).unwrap();

        let op = codegen.generate_output().unwrap();
        assert!(
            op.module().as_operation().verify(),
            "Top level module failed verification"
        );
        let op_str = format!("{}", op);
        similar_asserts::assert_eq!(
            op_str,
            r#"module attributes {veridise.lang = "llzk"} {
  struct.def @Signal<[]> {
    struct.field @reg : !felt.type {llzk.pub}
    function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Signal<[]>>
      struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
      function.return %self : !struct.type<@Signal<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint} {
      function.return
    }
  }
  struct.def @Main<[]> {
    function.def @compute(%arg0: !struct.type<@Signal<[]>>, %arg1: !struct.type<@Signal<[]>>, %arg2: !struct.type<@Signal<[]>>) -> !struct.type<@Main<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Main<[]>>
      function.return %self : !struct.type<@Main<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Main<[]>>, %arg1: !struct.type<@Signal<[]>>, %arg2: !struct.type<@Signal<[]>>, %arg3: !struct.type<@Signal<[]>>) attributes {function.allow_constraint} {
      function.return
    }
  }
}
"#
        );
    }

    #[rstest]
    fn define_main_function_public_outputs(ctx: LlzkContext) {
        let state: LlzkCodegenState = LlzkParamsBuilder::new(&ctx).no_optimize().build().into();
        let codegen = LlzkCodegen::initialize(&state);
        let mut cs = ConstraintSystem::<Fr>::default();
        let instance_col = cs.instance_column();
        let advice_io = AdviceIO::empty();
        let instance_io = InstanceIO::new(&[], &[(instance_col, &[0, 1, 2])]);
        let main = codegen
            .define_main_function(&advice_io, &instance_io)
            .unwrap();
        codegen.on_scope_end(main).unwrap();

        let op = codegen.generate_output().unwrap();
        assert!(
            op.module().as_operation().verify(),
            "Top level module failed verification"
        );
        let op_str = format!("{}", op);
        similar_asserts::assert_eq!(
            op_str,
            r#"module attributes {veridise.lang = "llzk"} {
  struct.def @Signal<[]> {
    struct.field @reg : !felt.type {llzk.pub}
    function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Signal<[]>>
      struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
      function.return %self : !struct.type<@Signal<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint} {
      function.return
    }
  }
  struct.def @Main<[]> {
    struct.field @out_0 : !felt.type {llzk.pub}
    struct.field @out_1 : !felt.type {llzk.pub}
    struct.field @out_2 : !felt.type {llzk.pub}
    function.def @compute() -> !struct.type<@Main<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Main<[]>>
      function.return %self : !struct.type<@Main<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Main<[]>>) attributes {function.allow_constraint} {
      function.return
    }
  }
}
"#
        );
    }

    #[rstest]
    fn define_main_function_private_outputs(ctx: LlzkContext) {
        let state: LlzkCodegenState = LlzkParamsBuilder::new(&ctx).no_optimize().build().into();
        let codegen = LlzkCodegen::initialize(&state);
        let mut cs = ConstraintSystem::<Fr>::default();
        let advice_col = cs.advice_column();
        let advice_io = AdviceIO::new(&[], &[(advice_col, &[0, 1, 2])]);
        let instance_io = InstanceIO::new(&[], &[]);
        let main = codegen
            .define_main_function(&advice_io, &instance_io)
            .unwrap();
        codegen.on_scope_end(main).unwrap();

        let op = codegen.generate_output().unwrap();
        assert!(
            op.module().as_operation().verify(),
            "Top level module failed verification"
        );
        let op_str = format!("{}", op);
        similar_asserts::assert_eq!(
            op_str,
            r#"module attributes {veridise.lang = "llzk"} {
  struct.def @Signal<[]> {
    struct.field @reg : !felt.type {llzk.pub}
    function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Signal<[]>>
      struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
      function.return %self : !struct.type<@Signal<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint} {
      function.return
    }
  }
  struct.def @Main<[]> {
    struct.field @out_0 : !felt.type
    struct.field @out_1 : !felt.type
    struct.field @out_2 : !felt.type
    function.def @compute() -> !struct.type<@Main<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Main<[]>>
      function.return %self : !struct.type<@Main<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Main<[]>>) attributes {function.allow_constraint} {
      function.return
    }
  }
}
"#
        );
    }

    #[rstest]
    fn define_main_function_mixed_io(ctx: LlzkContext) {
        let state: LlzkCodegenState = LlzkParamsBuilder::new(&ctx).no_optimize().build().into();
        let codegen = LlzkCodegen::initialize(&state);
        let mut cs = ConstraintSystem::<Fr>::default();
        let advice_col = cs.advice_column();
        let instance_col = cs.instance_column();
        let advice_io = AdviceIO::new(&[(advice_col, &[0, 1, 2])], &[(advice_col, &[3, 4])]);
        let instance_io = InstanceIO::new(&[(instance_col, &[0, 1])], &[(instance_col, &[2, 3])]);
        let main = codegen
            .define_main_function(&advice_io, &instance_io)
            .unwrap();
        codegen.on_scope_end(main).unwrap();

        let op = codegen.generate_output().unwrap();
        assert!(
            op.module().as_operation().verify(),
            "Top level module failed verification"
        );
        let op_str = format!("{}", op);
        similar_asserts::assert_eq!(
            op_str,
            r#"module attributes {veridise.lang = "llzk"} {
  struct.def @Signal<[]> {
    struct.field @reg : !felt.type {llzk.pub}
    function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Signal<[]>>
      struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
      function.return %self : !struct.type<@Signal<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint} {
      function.return
    }
  }
  struct.def @Main<[]> {
    struct.field @out_0 : !felt.type {llzk.pub}
    struct.field @out_1 : !felt.type {llzk.pub}
    struct.field @out_2 : !felt.type
    struct.field @out_3 : !felt.type
    function.def @compute(%arg0: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg1: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg2: !struct.type<@Signal<[]>>, %arg3: !struct.type<@Signal<[]>>, %arg4: !struct.type<@Signal<[]>>) -> !struct.type<@Main<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Main<[]>>
      function.return %self : !struct.type<@Main<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Main<[]>>, %arg1: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg2: !struct.type<@Signal<[]>> {llzk.pub = #llzk.pub}, %arg3: !struct.type<@Signal<[]>>, %arg4: !struct.type<@Signal<[]>>, %arg5: !struct.type<@Signal<[]>>) attributes {function.allow_constraint} {
      function.return
    }
  }
}
"#
        );
    }
}
