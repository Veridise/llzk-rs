use std::{cell::RefCell, iter, marker::PhantomData, ops::RangeFrom};

use anyhow::Result;
use counter::Counter;
use llzk::{
    dialect::{
        felt::FeltType,
        function::{self, FuncDefOpLike as _},
        module,
        r#struct::{self, FieldDefOp, StructDefOp, StructDefOpRef, StructType},
    },
    error::Error,
};
use lowering::LlzkStructLowering;
use melior::{
    ir::{
        attribute::FlatSymbolRefAttribute, operation::OperationLike as _, r#type::FunctionType,
        Block, BlockLike as _, Location, Module, Operation, OperationRef, Region, RegionLike as _,
        Type,
    },
    Context,
};
use midnight_halo2_proofs::plonk::Selector;
use ouroboros::self_referencing;

use crate::{gates::AnyQuery, ir::lift::LiftIRGuard, synthesis::CircuitSynthesis, LiftLike};

use super::{Backend, Codegen};

mod counter;
mod factory;
mod lowering;

#[derive(Default)]
pub struct LlzkParams {
    top_level: Option<String>,
    lift_fixed: bool,
}

#[self_referencing]
struct LlzkOutputInner {
    context: Context,
    #[not_covariant]
    #[borrows(context)]
    module: Module<'this>,
}

pub struct LlzkOutput {
    inner: LlzkOutputInner,
}

impl LlzkOutput {
    fn add_struct<'c, 'v>(&'c self, s: StructDefOp<'c>) -> Result<StructDefOpRef<'c, 'v>> {
        self.inner.with_module(move |module| {
            module
                .body()
                .append_operation(s.into())
                .try_into()
                .map_err(Into::into)
        })
    }
}

pub struct LlzkBackend<'a, 'c, 'v, F>
where
    'a: 'c,
{
    params: LlzkParams,
    output: LlzkOutput,
    struct_count: Counter,
    _lift_guard: LiftIRGuard<'a>,
    _marker: PhantomData<(F, OperationRef<'c, 'v>)>,
}

impl<'c, 'v, F: LiftLike> LlzkBackend<'_, 'c, 'v, F> {
    fn context(&self) -> &Context {
        self.output.inner.borrow_context()
    }
}

impl<'c, 'v: 'c, F: LiftLike> Codegen<'c> for LlzkBackend<'_, 'c, 'v, F> {
    type FuncOutput = LlzkStructLowering<'c, 'v, F>;

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

    fn define_main_function<'a: 'c>(
        &'a self,
        syn: &CircuitSynthesis<Self::F>,
    ) -> Result<Self::FuncOutput> {
        let advice_io = syn.advice_io();
        let instance_io = syn.instance_io();
        let s = factory::create_struct(
            self.context(),
            "Main",
            self.struct_count.next(),
            advice_io.inputs().len(),
            instance_io.inputs().len(),
            advice_io.outputs().len(),
            instance_io.outputs().len(),
        )?;
        Ok(LlzkStructLowering::new(self.output.add_struct(s)?))
    }
}

impl<'g, 'c, 'v: 'c, F: LiftLike> Backend<'c, LlzkParams, LlzkOutput>
    for LlzkBackend<'g, 'c, 'v, F>
{
    fn initialize(params: LlzkParams) -> Self {
        let enable_lifting = params.lift_fixed;
        let inner = LlzkOutputInner::new(Context::new(), |context| {
            let loc = match &params.top_level {
                Some(s) => Location::new(context, s, 0, 0),
                None => Location::unknown(context),
            };
            module::llzk_module(loc)
        });
        let output = LlzkOutput { inner };
        Self {
            output,
            params,
            struct_count: Default::default(),
            _lift_guard: LiftIRGuard::lock(enable_lifting),
            _marker: PhantomData,
        }
    }

    fn generate_output(self) -> Result<LlzkOutput> {
        Ok(self.output)
    }
}
