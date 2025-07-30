use crate::{synthesis::CircuitSynthesis, CircuitWithIO};
use anyhow::Result;

pub mod codegen;
pub mod events;
pub mod func;
pub mod llzk;
pub mod lowering;
pub mod picus;
pub mod resolvers;

use codegen::{
    lookup::codegen::LookupAsRowConstraint, strats::inline::InlineConstraintsStrat, Codegen,
    CodegenStrategy,
};
use resolvers::{QueryResolver, SelectorResolver};

pub trait Backend<'c, Params: Default>: Sized {
    type Codegen: Codegen<'c>;

    fn initialize(params: Params) -> Self;

    fn create_codegen(&'c self) -> Self::Codegen;

    /// Generate code using the default strategy.
    fn codegen<C>(&'c self, circuit: &C) -> Result<<Self::Codegen as Codegen<'c>>::Output>
    where
        C: CircuitWithIO<<Self::Codegen as Codegen<'c>>::F>,
        Self: 'c,
    {
        self.codegen_with_strat::<C, InlineConstraintsStrat<LookupAsRowConstraint>>(circuit)
    }

    /// Generate code using the given strategy.
    fn codegen_with_strat<'a, C, S>(
        &'c self,
        circuit: &C,
    ) -> Result<<Self::Codegen as Codegen<'c>>::Output>
    where
        C: CircuitWithIO<<Self::Codegen as Codegen<'c>>::F>,
        S: CodegenStrategy,
        Self: 'c,
    {
        let syn = CircuitSynthesis::new(circuit)?;

        let codegen = self.create_codegen();
        S::default().codegen(&codegen, &syn)?;

        codegen.generate_output()
    }
}
