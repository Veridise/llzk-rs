mod constraint;
mod matrix;
pub mod regions;

use std::collections::hash_set::Iter;

use anyhow::Result;
use constraint::{EqConstraint, Graph};
use regions::{RegionData, RegionRow, Regions};

use crate::{
    gates::find_gate_selector_set,
    halo2::*,
    io::{AdviceIOValidator, InstanceIOValidator},
    CircuitIO, CircuitWithIO,
};

pub struct CircuitSynthesis<F: Field> {
    cs: ConstraintSystem<F>,
    regions: Regions<F>,
    #[cfg(feature = "phase-tracking")]
    current_phase: sealed::Phase,

    eq_constraints: Graph<EqConstraint>,
    advice_io: CircuitIO<Advice>,
    instance_io: CircuitIO<Instance>,
}

impl<F: Field> CircuitSynthesis<F> {
    fn init<C: CircuitWithIO<F>>() -> (Self, C::Config) {
        let mut cs = ConstraintSystem::default();
        let config = C::configure(&mut cs);

        (
            Self {
                cs,
                regions: Default::default(),
                #[cfg(feature = "phase-tracking")]
                current_phase: FirstPhase.to_sealed(),
                advice_io: C::advice_io(&config),
                instance_io: C::instance_io(&config),
                eq_constraints: Default::default(),
            },
            config,
        )
    }

    pub fn new<C: CircuitWithIO<F>>(circuit: &C) -> Result<Self> {
        let (mut syn, config) = Self::init::<C>();

        syn.synthesize(circuit, config)?;

        syn.advice_io.validate(&AdviceIOValidator)?;
        syn.instance_io
            .validate(&InstanceIOValidator::new(&syn.cs))?;
        Ok(syn)
    }

    pub fn gates(&self) -> &[Gate<F>] {
        self.cs.gates()
    }

    pub fn regions(&self) -> &[RegionData<F>] {
        self.regions.regions()
    }

    pub fn advice_io(&self) -> &CircuitIO<Advice> {
        &self.advice_io
    }

    pub fn instance_io(&self) -> &CircuitIO<Instance> {
        &self.instance_io
    }

    pub fn constraints<'a>(&'a self) -> Iter<'a, ((Column<Any>, usize), (Column<Any>, usize))> {
        self.eq_constraints.iter()
    }

    pub fn region_gates<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a Gate<F>, RegionRow<'a, 'a, F>)> + 'a {
        self.regions()
            .iter()
            .flat_map(|region| {
                region
                    .rows()
                    .map(|row| RegionRow::new(region, row, self.advice_io(), self.instance_io()))
            })
            .flat_map(|r| {
                self.gates().iter().filter_map(move |gate| {
                    let selectors = find_gate_selector_set(gate.polynomials());
                    if r.gate_is_disabled(&selectors) {
                        return None;
                    }
                    Some((gate, r))
                })
            })
    }
}

#[cfg(not(feature = "phase-tracking"))]
impl<F: Field> CircuitSynthesis<F> {
    fn synthesize<C: Circuit<F>>(&mut self, circuit: &C, config: C::Config) -> Result<()> {
        let constants = self.cs.constants().clone();
        C::FloorPlanner::synthesize(self, circuit, config, constants)?;

        Ok(())
    }

    fn in_phase<P: Phase>(&self, _phase: P) -> bool {
        true
    }
}

#[cfg(feature = "phase-tracking")]
impl<F: Field> CircuitSynthesis<F> {
    fn synthesize<C: Circuit<F>>(&mut self, circuit: &C, config: C::Config) -> Result<()> {
        for current_phase in self.cs.phases() {
            self.current_phase = current_phase;

            C::FloorPlanner::synthesize(self, circuit, config.clone(), self.cs.constants.clone())?;
        }
        Ok(())
    }

    fn in_phase<P: Phase>(&self, phase: P) -> bool {
        self.current_phase == phase.to_sealed()
    }
}

impl<F: Field> Assignment<F> for CircuitSynthesis<F> {
    fn enter_region<NR, N>(&mut self, region_name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        if self.in_phase(FirstPhase) {
            self.regions.push(region_name);
        }
    }

    fn exit_region(&mut self) {
        if self.in_phase(FirstPhase) {
            self.regions.commit();
        }
    }

    fn enable_selector<A, AR>(&mut self, _: A, selector: &Selector, row: usize) -> Result<(), Error>
    where
        AR: Into<String>,
        A: FnOnce() -> AR,
    {
        self.regions.edit(|region| {
            region.enable_selector(selector.clone(), row);
        });
        Ok(())
    }

    fn query_instance(&self, _column: Column<Instance>, _row: usize) -> Result<Value<F>, Error> {
        Ok(Value::unknown())
    }

    fn assign_advice<V, VR, A, AR>(
        &mut self,
        name: A,
        advice: Column<Advice>,
        row: usize,
        _value: V,
    ) -> Result<(), Error>
    where
        VR: Into<Assigned<F>>,
        AR: Into<String>,
        V: FnOnce() -> Value<VR>,
        A: FnOnce() -> AR,
    {
        self.regions.edit(|region| {
            region.note_advice(advice, row, name().into());
            region.update_extent(advice.clone().into(), row);
        });
        Ok(())
    }

    fn assign_fixed<V, VR, A, AR>(
        &mut self,
        _: A,
        fixed: Column<Fixed>,
        row: usize,
        value: V,
    ) -> Result<(), Error>
    where
        VR: Into<Assigned<F>>,
        AR: Into<String>,
        V: FnOnce() -> Value<VR>,
        A: FnOnce() -> AR,
    {
        self.regions.edit(|region| {
            region.update_extent(fixed.into(), row);
            region.assign_fixed(fixed, row, value());
        });
        Ok(())
    }

    fn copy(
        &mut self,
        from: Column<Any>,
        from_row: usize,
        to: Column<Any>,
        to_row: usize,
    ) -> Result<(), Error> {
        assert!(*from.column_type() != Any::Fixed, "todo");
        assert!(*to.column_type() != Any::Fixed, "todo");
        self.eq_constraints.add((from, from_row, to, to_row));
        Ok(())
    }

    fn fill_from_row(
        &mut self,
        col: Column<Fixed>,
        row: usize,
        value: Value<Assigned<F>>,
    ) -> Result<(), Error> {
        log::debug!("fill_from_row{:?}", (col, row, value));
        todo!()
    }

    fn push_namespace<NR, N>(&mut self, name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.regions.edit(|region| region.push_namespace(name));
    }

    fn pop_namespace(&mut self, name: Option<String>) {
        self.regions.edit(|region| region.pop_namespace(name));
    }

    #[cfg(feature = "annotate-column")]
    fn annotate_column<A, AR>(&mut self, _: A, _: Column<Any>)
    where
        AR: Into<String>,
        A: FnOnce() -> AR,
    {
        todo!()
    }

    #[cfg(feature = "get-challenge")]
    fn get_challenge(&self, _: Challenge) -> Value<F> {
        todo!()
    }
}
