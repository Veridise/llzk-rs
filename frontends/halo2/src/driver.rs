//! Driver struct for handling synthesis and lowering.

use std::collections::HashMap;

use crate::{
    backend::{
        llzk::{LlzkBackend, LlzkOutput, LlzkParams},
        picus::{PicusBackend, PicusOutput, PicusParams},
    },
    halo2::PrimeField,
    io::{AdviceIO, AdviceIOValidator, InstanceIO, InstanceIOValidator},
    ir::{
        generate::{generate_ir, IRGenParams},
        IRCtx, ResolvedIRCircuit, UnresolvedIRCircuit,
    },
    synthesis::{CircuitSynthesis, Synthesizer},
    CircuitCallbacks,
};

/// Controls the different lowering stages of circuits.
#[derive(Default, Debug)]
pub struct Driver {
    ir_ctxs: HashMap<usize, IRCtx>,
    id_count: usize,
}

impl Driver {
    /// Synthesizes a circuit .
    pub fn synthesize<F, C>(&mut self, circuit: &C) -> anyhow::Result<CircuitSynthesis<F>>
    where
        C: CircuitCallbacks<F>,
        F: PrimeField,
    {
        let mut syn = Synthesizer::new(self.next_id());
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
    pub fn generate_ir<'syn, 'drv, 'cb, 'sco, F>(
        &'drv mut self,
        syn: &'syn CircuitSynthesis<F>,
        params: IRGenParams<'cb, '_, F>,
    ) -> anyhow::Result<UnresolvedIRCircuit<'drv, 'syn, 'sco, F>>
    where
        F: PrimeField,
        'syn: 'sco,
        'drv: 'sco + 'syn,
        'cb: 'sco + 'syn,
    {
        let ctx = self.get_or_create_ir_ctx(syn);
        let ir = generate_ir(syn, params, ctx)?;
        let enumerated_groups = syn.groups().iter().enumerate().collect::<Vec<_>>();
        let mut regions_to_groups = vec![];

        for (idx, group) in &enumerated_groups {
            for region in group.regions() {
                regions_to_groups.push((region.index().unwrap(), *idx));
            }
        }
        regions_to_groups.sort_by_key(|(ri, _)| **ri);
        debug_assert!(regions_to_groups
            .iter()
            .enumerate()
            .all(|(n, (ri, _))| n == **ri));
        let regions_to_groups = regions_to_groups
            .into_iter()
            .map(|(_, gidx)| gidx)
            .collect();
        Ok(UnresolvedIRCircuit::new(ctx, ir, regions_to_groups))
    }

    /// Creates the IR context for the synthesized circuit.
    fn get_or_create_ir_ctx<'drv, F: PrimeField>(
        &'drv mut self,
        syn: &CircuitSynthesis<F>,
    ) -> &'drv IRCtx {
        self.ir_ctxs
            .entry(syn.id())
            .or_insert_with(|| IRCtx::new(syn))
    }

    /// Creates a picus program from the circuit synthesis.
    pub fn picus(
        &self,
        ir: &ResolvedIRCircuit,
        params: PicusParams,
    ) -> anyhow::Result<PicusOutput> {
        PicusBackend::initialize(params).codegen(ir, ir.ctx())
    }

    /// Creates a llzk module from the circuit synthesis.
    pub fn llzk<'c>(
        &self,
        ir: &ResolvedIRCircuit,
        params: LlzkParams<'c>,
    ) -> anyhow::Result<LlzkOutput<'c>> {
        LlzkBackend::initialize(params).codegen(ir, ir.ctx())
    }

    fn next_id(&mut self) -> usize {
        let id = self.id_count;
        self.id_count += 1;
        id
    }
}
