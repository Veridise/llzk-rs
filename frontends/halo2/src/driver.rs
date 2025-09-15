use crate::{
    backend::{
        llzk::{LlzkBackend, LlzkOutput, LlzkParams},
        picus::{PicusBackend, PicusOutput, PicusParams},
    },
    gates::DefaultGateCallbacks,
    halo2::{Circuit, Field, PrimeField},
    io::{AdviceIOValidator, InstanceIOValidator},
    ir::{expr::IRAexpr, generate::generate_ir, IRCircuit, IRCtx, UnresolvedIRCircuit},
    lookups::callbacks::{DefaultLookupCallbacks, LookupCallbacks},
    synthesis::{CircuitSynthesis, Synthesizer},
    CircuitCallbacks, GateCallbacks,
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

    pub fn generate_ir<'s>(
        &self,
        syn: &'s CircuitSynthesis<F>,
        ctx: &'s IRCtx,
    ) -> anyhow::Result<UnresolvedIRCircuit<'s, F>> {
        generate_ir(syn, self.lookup_callbacks(), self.gate_callbacks(), ctx)
    }

    pub fn create_ir_ctx<'s>(&self, syn: &'s CircuitSynthesis<F>) -> anyhow::Result<IRCtx<'s>> {
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
