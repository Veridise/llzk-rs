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
    /// Sets the callbacks from the circuit's [`CircuitCallbacks`].
    ///
    /// If a callback is not configured by the circuit and the driver has a callback of that type
    /// configured already is not removed.
    pub fn set_callbacks<C>(&mut self)
    where
        C: Circuit<F> + CircuitCallbacks<F, C>,
    {
        if let Some(lookup) = C::lookup_callbacks() {
            self.lookup_callbacks = Some(lookup);
        }
        if let Some(gates) = C::gate_callbacks() {
            self.gate_callbacks = Some(gates);
        }
    }

    /// Synthesizes a circuit .
    pub fn synthesize<C>(&self, circuit: &C) -> anyhow::Result<CircuitSynthesis<F>>
    where
        C: Circuit<F> + CircuitCallbacks<F, C>,
    {
        let mut syn = Synthesizer::new();
        let config = C::configure(syn.cs_mut());

        let advice_io = C::advice_io(&config);
        let instance_io = C::instance_io(&config);
        log::debug!("Validating io hints");
        advice_io.validate(&AdviceIOValidator)?;
        instance_io.validate(&InstanceIOValidator::new(syn.cs()))?;

        log::debug!("Starting synthesis");
        let synthesis = syn.synthesize(circuit, config, advice_io, instance_io)?;
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

    /// Runs the Picus backend with the given strategy.
    #[cfg(test)]
    pub(crate) fn test_picus(
        &self,
        syn: CircuitSynthesis<F>,
        params: PicusParams,
        strat: impl crate::backend::codegen::CodegenStrategy,
    ) -> anyhow::Result<PicusOutput<F>> {
        PicusBackend::<F>::initialize(params).codegen_with_strat(
            syn,
            strat,
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

    /// Removes the lookup callbacks.
    pub fn remove_lookup_callbacks(&mut self) {
        self.lookup_callbacks = None;
    }

    /// Removes the gate callbacks.
    pub fn remove_gate_callbacks(&mut self) {
        self.gate_callbacks = None;
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
