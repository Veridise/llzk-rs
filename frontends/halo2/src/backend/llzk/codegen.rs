use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use super::lowering::LlzkStructLowering;
use super::state::LlzkCodegenState;
use super::{counter::Counter, LlzkOutput};
use anyhow::Result;
use llzk::dialect::{
    module::llzk_module,
    r#struct::{StructDefOp, StructDefOpRef},
};
use melior::ir::{BlockLike as _, Location, Module};
use melior::Context;
use midnight_halo2_proofs::plonk::Selector;

use crate::backend::codegen::queue::CodegenQueueHelper;
use crate::halo2::{Expression, RegionIndex};
use crate::ir::stmt::IRStmt;
use crate::LoweringField;
use crate::{gates::AnyQuery, synthesis::CircuitSynthesis};

use crate::backend::codegen::{Codegen, CodegenQueue};

use super::factory;

pub struct LlzkCodegen<'c, 's, F> {
    state: &'s LlzkCodegenState<'c, F>,
    queue: Rc<RefCell<CodegenQueueHelper<F>>>,
    module: Module<'c>,
    struct_count: Counter,
    _marker: PhantomData<F>,
}

impl<'c, F> LlzkCodegen<'c, '_, F> {
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

impl<'c: 's, 's, F: LoweringField> Codegen<'c, 's> for LlzkCodegen<'c, 's, F> {
    type FuncOutput = LlzkStructLowering<'c, F>;
    type Output = LlzkOutput<'c>;
    type F = F;
    type State = LlzkCodegenState<'c, F>;

    fn initialize(state: &'s Self::State) -> Self {
        Self {
            state,
            queue: Default::default(),
            module: llzk_module(Location::unknown(state.context())),
            struct_count: Default::default(),
            _marker: Default::default(),
        }
    }

    fn define_gate_function(
        &self,
        _name: &str,
        _selectors: &[&Selector],
        _input_queries: &[AnyQuery],
        _output_queries: &[AnyQuery],
        _syn: &CircuitSynthesis<Self::F>,
    ) -> Result<Self::FuncOutput> {
        unimplemented!()
    }

    fn define_main_function(&self, syn: &CircuitSynthesis<Self::F>) -> Result<Self::FuncOutput> {
        let advice_io = syn.advice_io();
        let instance_io = syn.instance_io();
        let struct_name = self.state.params().top_level().unwrap_or("Main");
        log::debug!("Creating struct with name '{struct_name}'");
        let s = factory::create_struct(
            self.context(),
            struct_name,
            self.struct_count.next(),
            (advice_io, instance_io),
        )?;
        log::debug!("Created struct object {s:?}");
        let regions = syn.regions_by_index();
        Ok(LlzkStructLowering::new(self.context(), s, Some(regions)))
    }

    fn define_function(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
        syn: Option<&CircuitSynthesis<Self::F>>,
    ) -> Result<Self::FuncOutput> {
        let s = factory::create_struct(
            self.context(),
            name,
            self.struct_count.next(),
            (inputs, outputs),
        )?;
        let regions = syn.map(|syn| syn.regions_by_index());
        Ok(LlzkStructLowering::new(self.context(), s, regions))
    }

    fn on_scope_end(&self, fo: Self::FuncOutput) -> Result<()> {
        self.queue.borrow_mut().dequeue_stmts(&fo)?;
        self.add_struct(fo.take_struct())?;
        Ok(())
    }

    fn generate_output(self) -> Result<Self::Output> {
        Ok(self.module.into())
    }
}

impl<'c: 's, 's, F: LoweringField> CodegenQueue<'c, 's> for LlzkCodegen<'c, 's, F> {
    fn enqueue_stmts(
        &self,
        region: RegionIndex,
        stmts: Vec<IRStmt<Expression<Self::F>>>,
    ) -> Result<()> {
        self.queue.borrow_mut().enqueue_stmts(region, stmts)
    }
}
