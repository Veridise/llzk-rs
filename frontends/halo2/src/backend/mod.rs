
use crate::{
    synthesis::CircuitSynthesis,
    CircuitWithIO,
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
use resolvers::{QueryResolver, SelectorResolver};

pub trait Backend<'c, Params: Default, Output>: Sized {
    type Codegen: Codegen<'c>;

    fn initialize(params: Params) -> Self;

    fn generate_output(self) -> Result<Output>;

    fn create_codegen(&self) -> Self::Codegen;

    /// Generate code using the given strategy.
    fn codegen<C>(self, circuit: &C) -> Result<Output>
    where
        C: CircuitWithIO<<Self::Codegen as Codegen<'c>>::F>,
    {
        self.codegen_with_strat::<C, InlineConstraintsStrat>(circuit)
    }

    /// Generate code using the given strategy.
    fn codegen_with_strat<C, S>(self, circuit: &C) -> Result<Output>
    where
        C: CircuitWithIO<<Self::Codegen as Codegen<'c>>::F>,
        S: CodegenStrategy,
    {
        let syn = CircuitSynthesis::new(circuit)?;

        let codegen = self.create_codegen();
        S::default().codegen(&codegen, &syn)?;

        self.generate_output()
    }
}
