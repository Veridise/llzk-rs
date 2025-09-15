//! Driver struct for handling synthesis and lowering.

use crate::{
    backend::{
        llzk::{LlzkBackend, LlzkOutput, LlzkParams},
        picus::{PicusBackend, PicusOutput, PicusParams},
    },
    gates::DefaultGateCallbacks,
    halo2::PrimeField,
    io::{AdviceIO, AdviceIOValidator, InstanceIO, InstanceIOValidator},
    ir::{expr::IRAexpr, generate::generate_ir, IRCircuit, IRCtx, UnresolvedIRCircuit},
    lookups::callbacks::{DefaultLookupCallbacks, LookupCallbacks},
    synthesis::{CircuitSynthesis, Synthesizer},
    CircuitCallbacks, GateCallbacks,
};

/// Controls the different lowering stages of circuits.
#[derive(Default, Debug)]
pub struct Driver {}

impl Driver {
    /// Synthesizes a circuit .
    pub fn synthesize<F, C>(&self, circuit: &C) -> anyhow::Result<CircuitSynthesis<F>>
    where
        C: CircuitCallbacks<F>,
        F: PrimeField,
    {
        let mut syn = Synthesizer::new();
        let config = C::configure(syn.cs_mut());

        let advice_io: AdviceIO = C::advice_io(&config);
        let instance_io: InstanceIO = C::instance_io(&config);
        log::debug!("Validating io hints");
        advice_io.validate(&AdviceIOValidator)?;
        instance_io.validate(&InstanceIOValidator::new(syn.cs()))?;

        log::debug!("Starting synthesis");
        let synthesis = syn.synthesize(circuit, config, advice_io, instance_io)?;
        log::debug!("Synthesis completed successfuly");
        Ok(synthesis)
    }

    /// Generates the IR of the synthesized circuit.
    pub fn generate_ir<'s, F: PrimeField>(
        &self,
        syn: &'s CircuitSynthesis<F>,
        ctx: &'s IRCtx,
        lookups: Option<&dyn LookupCallbacks<F>>,
        gates: Option<&dyn GateCallbacks<F>>,
    ) -> anyhow::Result<UnresolvedIRCircuit<'s, F>> {
        generate_ir(
            syn,
            lookups.unwrap_or(&DefaultLookupCallbacks),
            gates.unwrap_or(&DefaultGateCallbacks),
            ctx,
        )
    }

    /// Creates the IR context for the synthesized circuit.
    pub fn create_ir_ctx<'s, F: PrimeField>(
        &self,
        syn: &'s CircuitSynthesis<F>,
    ) -> anyhow::Result<IRCtx<'s>> {
        IRCtx::new(syn)
    }

    /// Creates a picus program from the circuit synthesis.
    pub fn picus(
        &self,
        ir: &IRCircuit<IRAexpr>,
        ctx: &IRCtx,
        params: PicusParams,
    ) -> anyhow::Result<PicusOutput> {
        PicusBackend::initialize(params).codegen(ir, ctx)
    }

    /// Creates a llzk module from the circuit synthesis.
    pub fn llzk<'c>(
        &self,
        ir: &IRCircuit<IRAexpr>,
        ctx: &IRCtx,
        params: LlzkParams<'c>,
    ) -> anyhow::Result<LlzkOutput<'c>> {
        LlzkBackend::initialize(params).codegen(ir, ctx)
    }
}
