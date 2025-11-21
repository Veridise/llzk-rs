use std::marker::PhantomData;

use crate::{
    backend::codegen::{CodegenParams, strats::inline::InlineConstraintsStrat},
    ir::{IRCtx, ResolvedIRCircuit},
};
use anyhow::Result;

pub use haloumi_ir_base::func;
pub use haloumi_lowering as lowering;
pub mod codegen;
//pub mod func;
#[cfg(feature = "llzk-backend")]
pub mod llzk;
#[cfg(feature = "picus-backend")]
pub mod picus;

use codegen::{Codegen, CodegenStrategy, strats::groups::GroupConstraintsStrat};

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
    C::State: CodegenParams,
{
    fn create_codegen(&'b self) -> C {
        C::initialize(&self.state)
    }

    /// Generate code using the default strategy.
    pub fn codegen(&'b self, ir: &ResolvedIRCircuit, ctx: &IRCtx) -> Result<C::Output> {
        if self.state.inlining_enabled() {
            self.codegen_with_strat(ir, ctx, InlineConstraintsStrat::default())
        } else {
            self.codegen_with_strat(ir, ctx, GroupConstraintsStrat::default())
        }
    }

    /// Generate code using the given strategy.
    fn codegen_with_strat(
        &'b self,
        ir: &ResolvedIRCircuit,
        ctx: &IRCtx,
        strat: impl CodegenStrategy,
    ) -> Result<C::Output> {
        log::debug!("Initializing code generator");
        let codegen = self.create_codegen();
        codegen.set_prime_field(ir.prime())?;
        log::debug!(
            "Starting code generation with {} strategy...",
            std::any::type_name_of_val(&strat)
        );

        strat.codegen(&codegen, ctx, ir)?;

        log::debug!("Code generation completed");
        codegen.generate_output()
    }
}
