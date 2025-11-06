use super::MulChip;
use crate::mul::MulConfig;
use ff::Field;
use midnight_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    default_group_key,
    plonk::{Circuit, ConstraintSystem, Error},
};
use std::marker::PhantomData;

pub mod deep_callstack;
pub mod different_bodies;
pub mod same_body;

#[derive(Default)]
pub struct MulCircuit<F>(pub PhantomData<F>);

impl<F: Field> Circuit<F> for MulCircuit<F> {
    type Config = MulConfig;
    type FloorPlanner = SimpleFloorPlanner;
    type Params = ();

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        MulChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = MulChip::construct(config);
        let prev_c = layouter.group(
            || "test group",
            default_group_key!(),
            |layouter, group| {
                let prev_c = chip.assign_first_row(layouter.namespace(|| "first row"))?;
                group.annotate_output(prev_c.cell())?;
                Ok(prev_c)
            },
        )?;
        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 1)?;

        Ok(())
    }
}
