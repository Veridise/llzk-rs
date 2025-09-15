use std::marker::PhantomData;

use crate::ir::{IRCircuit, IRCtx, expr::IRAexpr};
use anyhow::Result;

pub mod codegen;
pub mod func;
pub mod llzk;
pub mod lowering;
pub mod picus;

use codegen::{Codegen, CodegenStrategy, strats::groups::GroupConstraintsStrat};

//type DefaultStrat = InlineConstraintsStrat;
type DefaultStrat = GroupConstraintsStrat;

/// Entrypoint for the backend.
pub struct Backend<C, S> {
    state: S,
    _codegen: PhantomData<C>,
}

impl<C, S> Backend<C, S> {
    pub fn initialize<P: Clone + Into<S>>(params: P) -> Self {
        Self {
            state: params.into(),
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
