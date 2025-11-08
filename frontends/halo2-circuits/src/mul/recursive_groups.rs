use ff::Field;
use midnight_proofs::poly::Rotation;
use midnight_proofs::{
    circuit::{AssignedCell, Layouter, SimpleFloorPlanner},
    default_group_key,
    plonk::{Circuit, ConstraintSystem, Error},
};
use std::marker::PhantomData;

use crate::mul::MulConfig;

const N_INPUTS: usize = 4;

#[derive(Debug, Clone)]
struct MulChip<F: Field> {
    config: MulConfig,
    _marker: PhantomData<F>,
}

impl<F: Field> MulChip<F> {
    pub fn construct(config: MulConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> MulConfig {
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        meta.create_gate("mul", |meta| {
            //
            // col_fixed | col_a | col_b | col_c | selector
            //      f       a      b        c       s
            //
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());

            midnight_proofs::plonk::Constraints::with_selector(selector, vec![a * b - c])
        });

        MulConfig {
            col_fixed: meta.fixed_column(),
            col_a,
            col_b,
            col_c,
            selector,
            instance,
        }
    }

    pub fn expose_public(
        &self,
        layouter: &mut impl Layouter<F>,
        cell: &AssignedCell<F, F>,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, row)
    }

    pub fn mul_many(
        &self,
        layouter: &mut impl Layouter<F>,
        operands: &[AssignedCell<F, F>],
    ) -> Result<AssignedCell<F, F>, Error> {
        if operands.len() == 1 {
            return Ok(operands[0].clone());
        }
        layouter.group(
            || "mul_many",
            default_group_key!(),
            |layouter, group| {
                group.annotate_inputs(operands.iter().map(|op| op.cell()))?;
                let lhs = &operands[0];
                let rhs = self.mul_many(layouter, &operands[1..])?;
                layouter.assign_region(
                    || "mul",
                    |mut region| {
                        self.config.selector.enable(&mut region, 0)?;
                        assert!(operands.len() > 1);

                        let a = region.assign_advice(
                            || "a = lhs",
                            self.config.col_a,
                            0,
                            || lhs.value().copied(),
                        )?;
                        region.constrain_equal(lhs.cell(), a.cell())?;
                        let b = region.assign_advice(
                            || "b = rhs",
                            self.config.col_b,
                            0,
                            || rhs.value().copied(),
                        )?;
                        region.constrain_equal(rhs.cell(), b.cell())?;
                        let c = region.assign_advice(
                            || "a * b",
                            self.config.col_c,
                            0,
                            || a.value().copied() * b.value(),
                        )?;
                        group.annotate_output(c.cell())?;
                        Ok(c)
                    },
                )
            },
        )
    }

    pub fn assign_inputs(
        &self,
        layouter: &mut impl Layouter<F>,
        input_count: usize,
    ) -> Result<Vec<AssignedCell<F, F>>, Error> {
        layouter.assign_region(
            || "input loading",
            |mut region| {
                (0..input_count)
                    .map(|n| {
                        region.assign_advice_from_instance(
                            || format!("input {n}"),
                            self.config.instance,
                            n,
                            self.config.col_a,
                            n,
                        )
                    })
                    .collect()
            },
        )
    }
}

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
        let inputs = chip.assign_inputs(&mut layouter, N_INPUTS)?;
        let output = chip.mul_many(&mut layouter, &inputs)?;
        chip.expose_public(&mut layouter, &output, inputs.len())?;

        Ok(())
    }
}
