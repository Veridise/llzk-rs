use std::marker::PhantomData;

use anyhow::Result;
use counter::Counter;
use llzk::dialect::r#struct::{StructDefOp, StructDefOpRef};
use lowering::LlzkStructLowering;
use melior::{
    ir::{
        operation::OperationLike as _, BlockLike as _, Module,
    },
    Context,
};
use midnight_halo2_proofs::plonk::Selector;
use ouroboros::self_referencing;

use crate::{gates::AnyQuery, ir::lift::LiftIRGuard, synthesis::CircuitSynthesis, LiftLike};

use super::{Backend, Codegen};

mod codegen;
mod counter;
mod extras;
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
    fn module<'c, 'v: 'c>(&'v self) -> &'v Module<'c> {
        self.inner.with_module(move |module| module)
    }
}

pub struct LlzkCodegen<'c, F> {
    module: Module<'c>,
    struct_count: Counter,
    _marker: PhantomData<F>,
}

impl<'c, F> LlzkCodegen<'c, F> {
    pub fn new(context: &Context) -> Self {
        Self {
            module: todo!(),
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

//impl<'c, 'v, F: LiftLike> LlzkBackend<'c, F> {
//    fn context(&self) -> &Context {
//        self.output.inner.borrow_context()
//    }
//}

impl<'c, 'v: 'c, F: LiftLike> Codegen<'c> for LlzkCodegen<'c, F> {
    type FuncOutput = LlzkStructLowering<'c, F>;

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
}

pub struct LlzkBackend<F> {
    params: LlzkParams,
    context: Context,
    _lift_guard: LiftIRGuard,
    _marker: PhantomData<F>,
}

impl<'c, 'v: 'c, F: LiftLike> Backend<'c, LlzkParams, LlzkOutput> for LlzkBackend<F> {
    type Codegen = LlzkCodegen<'c, F>;

    fn initialize(params: LlzkParams) -> Self {
        let enable_lifting = params.lift_fixed;
        //let inner = LlzkOutputInner::new(Context::new(), |context| {
        //    let loc = match &params.top_level {
        //        Some(s) => Location::new(context, s, 0, 0),
        //        None => Location::unknown(context),
        //    };
        //    module::llzk_module(loc)
        //});
        //let output = LlzkOutput { inner };
        let context = Context::new();
        Self {
            context,
            params,
            _lift_guard: LiftIRGuard::lock(enable_lifting),
            _marker: Default::default(),
        }
    }

    fn generate_output(self) -> Result<LlzkOutput> {
        todo!()
    }

    fn create_codegen(&self) -> Self::Codegen {
        LlzkCodegen::new(&self.context)
    }
}
