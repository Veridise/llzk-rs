use std::marker::PhantomData;

#[cfg(feature = "lift-field-operations")]
use crate::ir::lift::{LiftIRGuard, LiftingCfg};
use crate::{
    gates::DefaultGateCallbacks, halo2::Circuit, lookups::callbacks::DefaultLookupCallbacks,
    synthesis::CircuitSynthesis, CircuitCallbacks,
};
use anyhow::Result;

pub mod codegen;
pub mod events;
pub mod func;
pub mod llzk;
pub mod lowering;
pub mod picus;
pub mod resolvers;

use codegen::{strats::inline::InlineConstraintsStrat, Codegen, CodegenQueue, CodegenStrategy};
use events::BackendEventReceiver;
use resolvers::{QueryResolver, SelectorResolver};

type DefaultStrat = InlineConstraintsStrat;

pub struct Backend<C, S> {
    state: S,
    #[cfg(feature = "lift-field-operations")]
    _lift_guard: LiftIRGuard,
    _codegen: PhantomData<C>,
}

#[cfg(not(feature = "lift-field-operations"))]
impl<'s, C, S: 's> Backend<C, S> {
    pub fn initialize<P: Clone + Into<S> + 's>(params: P) -> Self {
        Self {
            state: params.into(),
            _codegen: PhantomData,
        }
    }
}

#[cfg(feature = "lift-field-operations")]
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

    pub fn event_receiver(&'b self) -> BackendEventReceiver<'b, C::F>
    where
        C: CodegenQueue<'s, 'b>,
    {
        BackendEventReceiver::new(C::event_receiver(&self.state))
    }

    /// Generate code using the default strategy.
    pub fn codegen<CR, CB>(&'b self, circuit: &CR) -> Result<C::Output>
    where
        CR: Circuit<C::F>,
        CB: CircuitCallbacks<C::F, CR>,
    {
        self.codegen_with_strat::<CR, CB, DefaultStrat>(circuit)
    }

    /// Generate code using the given strategy.
    pub(crate) fn codegen_with_strat<CR, CB, ST>(&'b self, circuit: &CR) -> Result<C::Output>
    where
        CR: Circuit<C::F>,
        CB: CircuitCallbacks<C::F, CR>,
        ST: CodegenStrategy,
    {
        let syn = CircuitSynthesis::new::<CR, CB>(circuit)?;
        let lookup_cbs = CB::lookup_callbacks().unwrap_or(Box::new(DefaultLookupCallbacks));
        let gate_cbs = CB::gate_callbacks().unwrap_or(Box::new(DefaultGateCallbacks));

        log::debug!("Initializing code generator");
        let codegen = self.create_codegen();
        log::debug!("Starting codegen...");
        ST::default().codegen(&codegen, &syn, &*lookup_cbs, &*gate_cbs)?;

        log::debug!("Codegen completed");
        codegen.generate_output()
    }
}
