use std::marker::PhantomData;

use anyhow::Result;
use codegen::LlzkCodegen;
use counter::Counter;
use llzk::dialect::r#struct::{StructDefOp, StructDefOpRef};
use lowering::LlzkStructLowering;
use melior::{
    ir::{operation::OperationLike as _, BlockLike as _, Module},
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

pub struct LlzkBackend<F> {
    params: LlzkParams,
    context: Context,
    _lift_guard: LiftIRGuard,
    _marker: PhantomData<F>,
}

impl<'c, F: LiftLike> Backend<'c, LlzkParams> for LlzkBackend<F> {
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

    fn create_codegen(&'c self) -> Self::Codegen {
        LlzkCodegen::new(&self.context)
    }
}
