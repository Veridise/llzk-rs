use std::marker::PhantomData;

use crate::halo2::PrimeField;
use codegen::LlzkCodegen;
use melior::{
    ir::{operation::OperationLike as _, Module},
    Context,
};
use ouroboros::self_referencing;

#[cfg(feature = "lift-field-operations")]
use crate::{ir::lift::LiftIRGuard, LiftLike};

use super::Backend;

mod codegen;
mod counter;
mod extras;
mod factory;
mod lowering;

#[derive(Default)]
pub struct LlzkParams {
    top_level: Option<String>,
    #[cfg(feature = "lift-field-operations")]
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
    #[cfg(feature = "lift-field-operations")]
    _lift_guard: LiftIRGuard,
    _marker: PhantomData<F>,
}

#[cfg(feature = "lift-field-operations")]
trait LlzkPrimeField: LiftLike {}
#[cfg(feature = "lift-field-operations")]
impl<F: LiftLike> LlzkPrimeField for F {}
#[cfg(not(feature = "lift-field-operations"))]
trait LlzkPrimeField: PrimeField {}
#[cfg(not(feature = "lift-field-operations"))]
impl<F: PrimeField> LlzkPrimeField for F {}

impl<'c, F: LlzkPrimeField> Backend<'c, LlzkParams> for LlzkBackend<F> {
    type Codegen = LlzkCodegen<'c, F>;

    fn initialize(params: LlzkParams) -> Self {
        #[cfg(feature = "lift-field-operations")]
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
            #[cfg(feature = "lift-field-operations")]
            _lift_guard: LiftIRGuard::lock(enable_lifting),
            _marker: Default::default(),
        }
    }

    fn create_codegen(&'c self) -> Self::Codegen {
        LlzkCodegen::new(&self.context)
    }
}
