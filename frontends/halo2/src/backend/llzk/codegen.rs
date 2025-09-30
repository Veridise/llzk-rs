use super::lowering::LlzkStructLowering;
use super::state::LlzkCodegenState;
use super::{counter::Counter, LlzkOutput};
use anyhow::Result;

use llzk::prelude::*;
use melior::ir::{BlockLike as _, Location, Module};
use melior::Context;

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

impl<'c> LlzkCodegen<'c, '_> {
    fn add_struct(&self, s: StructDefOp<'c>) -> Result<StructDefOpRef<'c, '_>> {
        self.module
            .body()
            .append_operation(s.into())
            .try_into()
            .map_err(Into::into)
    }

    fn context(&self) -> &'c Context {
        self.state.context()
    }
}

impl<'c: 's, 's> Codegen<'c, 's> for LlzkCodegen<'c, 's> {
    type FuncOutput = LlzkStructLowering<'c>;
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

    fn set_prime_field(&self, _prime: Felt) -> Result<()> {
        todo!()
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
        //let regions = syn.regions_by_index();
        Ok(LlzkStructLowering::new(self.context(), s))
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
        Ok(LlzkStructLowering::new(self.context(), s))
    }

    fn on_scope_end(&self, fo: Self::FuncOutput) -> Result<()> {
        self.add_struct(fo.take_struct())?;
        Ok(())
    }

    fn generate_output(self) -> Result<Self::Output> {
        let signal = r#struct::helpers::define_signal_struct(self.context())?;
        self.module.body().insert_operation(0, signal.into());
        Ok(self.module.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        halo2::{ConstraintSystem, Fr},
        LlzkParamsBuilder,
    };

    use super::*;
    use log::LevelFilter;
    use melior::diagnostic::{Diagnostic, DiagnosticSeverity};
    use rstest::{fixture, rstest};
    use simplelog::{Config, TestLogger};

    #[fixture]
    fn common() {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
    }

    /// Diagnostics handler that writes the diagnostics to the [`log`].
    pub fn diag_logger(diag: Diagnostic) -> bool {
        fn log_msg(diag: &Diagnostic) {
            match diag.severity() {
                DiagnosticSeverity::Error => {
                    log::error!("[{}] {}", diag.location(), diag.to_string())
                }
                DiagnosticSeverity::Note => {
                    log::info!("[{}] note: {}", diag.location(), diag.to_string())
                }
                DiagnosticSeverity::Remark => {
                    log::info!("[{}] remark: {}", diag.location(), diag.to_string())
                }
                DiagnosticSeverity::Warning => {
                    log::warn!("[{}] {}", diag.location(), diag.to_string())
                }
            }
        }
        fn log_notes(diag: &Diagnostic) -> Result<(), bool> {
            for note_no in 0..diag.note_count() {
                let note = diag.note(note_no);
                match note {
                    Ok(note) => {
                        log_msg(&note);
                        log_notes(&note)?;
                    }
                    Err(err) => {
                        log::error!("Error while obtaining note #{note_no}: {err}");
                        return Err(false);
                    }
                };
            }
            Ok(())
        }
        log_msg(&diag);
        if let Err(res) = log_notes(&diag) {
            return res;
        }

        match diag.severity() {
            DiagnosticSeverity::Error => false,
            _ => true,
        }
    }

    #[fixture]
    #[allow(unused_variables)]
    fn ctx(common: ()) -> LlzkContext {
        let context = LlzkContext::new();
        context.attach_diagnostic_handler(diag_logger);
        context
    }

    #[rstest]
    fn define_main_function_empty_io(ctx: LlzkContext) {
        let state: LlzkCodegenState = LlzkParamsBuilder::new(&ctx).build().into();
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
        let state: LlzkCodegenState = LlzkParamsBuilder::new(&ctx).build().into();
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
}
