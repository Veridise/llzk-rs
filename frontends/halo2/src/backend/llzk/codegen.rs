use std::marker::PhantomData;

use super::lowering::LlzkStructLowering;
use super::{counter::Counter, LlzkOutput};
use anyhow::Result;
use llzk::dialect::{
    module::llzk_module,
    r#struct::{StructDefOp, StructDefOpRef},
};
use melior::{
    ir::{BlockLike as _, Location, Module},
    Context,
};
use midnight_halo2_proofs::plonk::Selector;

use crate::{gates::AnyQuery, synthesis::CircuitSynthesis};

use crate::backend::codegen::Codegen;

use super::{factory, LlzkPrimeField};

pub struct LlzkCodegen<'c, F> {
    module: Module<'c>,
    struct_count: Counter,
    _marker: PhantomData<F>,
}

impl<'c, F> LlzkCodegen<'c, F> {
    pub fn new(context: &'c Context) -> Self {
        Self {
            module: llzk_module(Location::unknown(context)),
            struct_count: Default::default(),
            _marker: Default::default(),
        }
    }

    fn add_struct(&self, s: StructDefOp<'c>) -> Result<StructDefOpRef<'c, '_>> {
        self.module
            .body()
            .append_operation(s.into())
            .try_into()
            .map_err(Into::into)
    }
}

impl<'c, F: LlzkPrimeField> Codegen<'c> for LlzkCodegen<'c, F> {
    type FuncOutput = LlzkStructLowering<'c, F>;
    type Output = LlzkOutput;
    type F = F;

    fn define_gate_function(
        &self,
        _name: &str,
        _selectors: &[&Selector],
        _input_queries: &[AnyQuery],
        _output_queries: &[AnyQuery],
        _syn: &CircuitSynthesis<Self::F>,
    ) -> Result<Self::FuncOutput> {
        todo!()
    }

    fn define_main_function(&self, syn: &CircuitSynthesis<Self::F>) -> Result<Self::FuncOutput> {
        let advice_io = syn.advice_io();
        let instance_io = syn.instance_io();
        let s = factory::create_struct(
            unsafe { self.module.context().to_ref() },
            "Main",
            self.struct_count.next(),
            advice_io.inputs().len(),
            instance_io.inputs().len(),
            advice_io.outputs().len(),
            instance_io.outputs().len(),
        )?;
        Ok(LlzkStructLowering::new(s))
    }

    fn on_scope_end(&self, fo: Self::FuncOutput) -> Result<()> {
        self.add_struct(fo.take_struct())?;
        Ok(())
    }

    fn generate_output(self) -> Result<Self::Output> {
        todo!()
    }

    fn define_function(
        &self,
        _name: &str,
        _inputs: usize,
        _outputs: usize,
        _syn: Option<&CircuitSynthesis<Self::F>>,
    ) -> Result<Self::FuncOutput> {
        todo!()
    }
}
