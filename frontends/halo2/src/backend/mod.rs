use std::marker::PhantomData;

#[cfg(feature = "lift-field-operations")]
use crate::ir::lift::{LiftIRGuard, LiftingCfg};
use crate::{
    ir::{expr::IRAexpr, IRCircuit, IRCtx},
    lookups::callbacks::LookupCallbacks,
    synthesis::CircuitSynthesis,
    GateCallbacks,
};
use anyhow::Result;

pub mod codegen;
pub mod events;
pub mod func;
pub mod llzk;
pub mod lowering;
pub mod picus;
pub mod resolvers;

use codegen::{strats::groups::GroupConstraintsStrat, Codegen, CodegenStrategy};
use events::BackendEventReceiver;
use resolvers::{QueryResolver, SelectorResolver};

//type DefaultStrat = InlineConstraintsStrat;
type DefaultStrat = GroupConstraintsStrat;

/// Entrypoint for the backend.
pub struct Backend<C, S> {
    state: S,
    #[cfg(feature = "lift-field-operations")]
    _lift_guard: LiftIRGuard,
    _codegen: PhantomData<C>,
}

#[cfg(not(feature = "lift-field-operations"))]
#[allow(clippy::extra_unused_lifetimes)]
impl<'s, C, S: 's> Backend<C, S> {
    pub fn initialize<P: Clone + Into<S> + 's>(params: P) -> Self {
        Self {
            state: params.into(),
            _codegen: PhantomData,
        }
    }
}

#[cfg(feature = "lift-field-operations")]
#[allow(clippy::extra_unused_lifetimes)]
impl<'s, C, S: 's> Backend<C, S> {
    pub fn initialize<P: Clone + Into<S> + LiftingCfg + 's>(params: P) -> Self {
        let enable_lifting = params.lifting_enabled();
        Self {
            state: params.into(),
            _lift_guard: LiftIRGuard::lock(enable_lifting),
            _codegen: PhantomData,
        }
    }
}

impl<'b, 's: 'b, C> Backend<C, C::State>
where
    C: Codegen<'s, 'b>,
    C::State: 's,
    C::Output: 's,
{
    fn create_codegen(&'b self) -> C {
        C::initialize(&self.state)
    }

    //pub fn event_receiver(&'b self) -> BackendEventReceiver<'b, C::F>
    //where
    //    C: CodegenQueue<'s, 'b>,
    //{
    //    BackendEventReceiver::new(C::event_receiver(&self.state))
    //}

    /// Generate code using the default strategy.
    pub fn codegen(&'b self, ir: &IRCircuit<IRAexpr>, ctx: &IRCtx) -> Result<C::Output> {
        self.codegen_with_strat(ir, ctx, DefaultStrat::default())
    }

    /// Generate code using the given strategy.
    pub(crate) fn codegen_with_strat(
        &'b self,
        ir: &IRCircuit<IRAexpr>,
        ctx: &IRCtx,
        strat: impl CodegenStrategy,
    ) -> Result<C::Output> {
        log::debug!("Initializing code generator");
        let codegen = self.create_codegen();
        log::debug!(
            "Starting code generation with {} strategy...",
            std::any::type_name_of_val(&strat)
        );

        strat.codegen(&codegen, ctx, ir)?;

        log::debug!("Code generation completed");
        codegen.generate_output()
    }
}
