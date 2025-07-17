use std::{cell::RefCell, iter, marker::PhantomData, ops::RangeFrom};

use anyhow::Result;
use counter::Counter;
use llzk::{
    dialect::{
        felt::FeltType,
        function::{self, FuncDefOpLike as _},
        r#struct::{self, FieldDefOp, StructDefOp, StructType},
    },
    error::Error,
};
use lowering::LlzkStructLowering;
use melior::{
    ir::{
        attribute::FlatSymbolRefAttribute, r#type::FunctionType, Block, BlockLike as _, Location,
        Operation, Region, RegionLike as _, Type,
    },
    Context,
};
use midnight_halo2_proofs::plonk::Selector;

use crate::{gates::AnyQuery, ir::lift::LiftIRGuard, synthesis::CircuitSynthesis, LiftLike};

use super::Codegen;

mod counter;
mod factory;
mod lowering;

pub struct LlzkParams {}

pub struct LlzkBackend<'a, F> {
    context: Context,
    struct_count: Counter,
    _lift_guard: LiftIRGuard<'a>,
    _marker: PhantomData<F>,
}

impl<'c, F: LiftLike> Codegen<'c> for LlzkBackend<'c, F> {
    type FuncOutput = LlzkStructLowering<'c, 'c, F>;

    type F = F;

    fn define_gate_function<'f>(
        &self,
        name: &str,
        selectors: &[&Selector],
        queries: &[AnyQuery],
        syn: &CircuitSynthesis<Self::F>,
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        todo!()
    }

    fn define_main_function<'f>(&self, syn: &CircuitSynthesis<Self::F>) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        let advice_io = syn.advice_io();
        let instance_io = syn.instance_io();
        let s = factory::create_struct(
            &self.context,
            "Main",
            self.struct_count.next(),
            advice_io.inputs().len(),
            instance_io.inputs().len(),
            advice_io.outputs().len(),
            instance_io.outputs().len(),
        )?;
        todo!()
    }
}
