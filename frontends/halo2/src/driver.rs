use crate::{
    gates::DefaultGateCallbacks,
    halo2::{Circuit, Field, PrimeField},
    io::{AdviceIOValidator, InstanceIOValidator},
    lookups::callbacks::{DefaultLookupCallbacks, LookupCallbacks},
    synthesis::{CircuitSynthesis, Synthesizer},
    CircuitCallbacks, GateCallbacks, LlzkBackend, LlzkOutput, LlzkParams, PicusBackend,
    PicusOutput, PicusParams,
};

/// Controls the different lowering stages of circuits.
#[derive(Default)]
pub struct Driver<'lc, 'gc, F: Field> {
    lookup_callbacks: Option<Box<dyn LookupCallbacks<F> + 'lc>>,
    gate_callbacks: Option<Box<dyn GateCallbacks<F> + 'gc>>,
}

impl<'lc, 'gc, F: PrimeField> Driver<'lc, 'gc, F> {
    /// Synthesizes a circuit.
    pub fn synthesize<C, CB>(&self, circuit: &C) -> anyhow::Result<CircuitSynthesis<F>>
    where
        C: Circuit<F>,
        CB: CircuitCallbacks<F, C>,
    {
        let mut syn = Synthesizer::new();
        let config = C::configure(syn.cs_mut());

        let advice_io = CB::advice_io(&config);
        let instance_io = CB::instance_io(&config);
        log::debug!("Validating io hints");
        advice_io.validate(&AdviceIOValidator)?;
        instance_io.validate(&InstanceIOValidator::new(syn.cs()))?;

        log::debug!("Starting synthesis");
        let synthesis = Synthesizer::new().synthesize(circuit, config, advice_io, instance_io)?;
        log::debug!("Synthesis completed successfuly");
        Ok(synthesis)
    }

    /// Creates a picus program from the circuit synthesis.
    pub fn picus(
        &self,
        syn: CircuitSynthesis<F>,
        params: PicusParams,
    ) -> anyhow::Result<PicusOutput<F>> {
        PicusBackend::<F>::initialize(params).codegen(
            syn,
            self.lookup_callbacks(),
            self.gate_callbacks(),
        )
    }

    /// Creates a llzk module from the circuit synthesis.
    pub fn llzk<'c>(
        &self,
        syn: CircuitSynthesis<F>,
        params: LlzkParams<'c>,
    ) -> anyhow::Result<LlzkOutput<'c>> {
        LlzkBackend::<F>::initialize(params).codegen(
            syn,
            self.lookup_callbacks(),
            self.gate_callbacks(),
        )
    }

    /// Sets the lookup callbacks.
    pub fn set_lookup_callbacks(&mut self, cb: impl LookupCallbacks<F> + 'lc) {
        self.lookup_callbacks = Some(Box::new(cb));
    }

    /// Sets the gate callbacks.
    pub fn set_gate_callbacks(&mut self, cb: impl GateCallbacks<F> + 'gc) {
        self.gate_callbacks = Some(Box::new(cb));
    }

    /// Returns the configured lookup callbacks or the default if none were configured.
    fn lookup_callbacks(&self) -> &dyn LookupCallbacks<F> {
        self.lookup_callbacks
            .as_deref()
            .unwrap_or(&DefaultLookupCallbacks)
    }

    /// Returns the configured gate callbacks or the default if none were configured.
    fn gate_callbacks(&self) -> &dyn GateCallbacks<F> {
        self.gate_callbacks
            .as_deref()
            .unwrap_or(&DefaultGateCallbacks)
    }
}
