use crate::{
    halo2::Circuit, lookups::callbacks::DefaultLookupCallbacks, synthesis::CircuitSynthesis,
    CircuitCallbacks,
};
use anyhow::Result;

pub mod codegen;
pub mod events;
pub mod func;
pub mod llzk;
pub mod lowering;
pub mod picus;
pub mod resolvers;

use codegen::{strats::inline::InlineConstraintsStrat, Codegen, CodegenStrategy};
use events::BackendEventReceiver;
use resolvers::{QueryResolver, SelectorResolver};

type DefaultStrat = InlineConstraintsStrat;

pub trait Backend<'c, Params: Default>: Sized {
    type Codegen: Codegen<'c>;

    fn initialize(params: Params) -> Self;

    fn create_codegen(&'c self) -> Self::Codegen;

    fn event_receiver(&self) -> BackendEventReceiver<<Self::Codegen as Codegen<'c>>::F>;

    /// Generate code using the default strategy.
    fn codegen<C, CB>(&'c self, circuit: &C) -> Result<<Self::Codegen as Codegen<'c>>::Output>
    where
        C: Circuit<<Self::Codegen as Codegen<'c>>::F>,
        CB: CircuitCallbacks<<Self::Codegen as Codegen<'c>>::F, C>,
        Self: 'c,
    {
        self.codegen_with_strat::<C, CB, DefaultStrat>(circuit)
    }

    /// Generate code using the given strategy.
    fn codegen_with_strat<'a, C, CB, S>(
        &'c self,
        circuit: &C,
    ) -> Result<<Self::Codegen as Codegen<'c>>::Output>
    where
        C: Circuit<<Self::Codegen as Codegen<'c>>::F>,
        CB: CircuitCallbacks<<Self::Codegen as Codegen<'c>>::F, C>,
        S: CodegenStrategy,
        Self: 'c,
    {
        let syn = CircuitSynthesis::new::<C, CB>(circuit)?;
        let lookup_cbs = CB::lookup_callbacks().unwrap_or(Box::new(DefaultLookupCallbacks));

        let codegen = self.create_codegen();
        S::default().codegen(&codegen, &syn, &*lookup_cbs)?;

        codegen.generate_output()
    }
}
