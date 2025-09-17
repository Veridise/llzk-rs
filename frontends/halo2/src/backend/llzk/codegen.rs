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
        Self {
            state,
            module: llzk_module(Location::unknown(state.context())),
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
        )?;
        Ok(LlzkStructLowering::new(self.context(), s))
    }

    fn on_scope_end(&self, fo: Self::FuncOutput) -> Result<()> {
        self.add_struct(fo.take_struct())?;
        Ok(())
    }

    fn generate_output(self) -> Result<Self::Output> {
        Ok(self.module.into())
    }
}
