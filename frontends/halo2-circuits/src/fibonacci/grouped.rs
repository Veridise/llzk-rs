use ff::Field;
use midnight_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner};
use midnight_proofs::default_group_key;
use midnight_proofs::plonk::{Circuit, ConstraintSystem, Error};
use std::marker::PhantomData;

use crate::fibonacci::{FibonacciConfig, fibonacci_gates};

#[derive(Debug, Clone)]
struct FibonacciChip<F: Field> {
    config: FibonacciConfig,
    _marker: PhantomData<F>,
}

type FibNums<F> = (AssignedCell<F, F>, AssignedCell<F, F>);

impl<F: Field> FibonacciChip<F> {
    pub fn construct(config: FibonacciConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> FibonacciConfig {
        fibonacci_gates(meta)
    }

    #[allow(clippy::type_complexity)]
    pub fn assign_inputs(&self, layouter: &mut impl Layouter<F>) -> Result<FibNums<F>, Error> {
        layouter.assign_region(
            || "assign inputs",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;

                let a_cell = region.assign_advice_from_instance(
                    || "f(0)",
                    self.config.instance,
                    0,
                    self.config.col_a,
                    0,
                )?;

                let b_cell = region.assign_advice_from_instance(
                    || "f(1)",
                    self.config.instance,
                    1,
                    self.config.col_b,
                    0,
                )?;

                Ok((a_cell, b_cell))
            },
        )
    }

    pub fn step(
        &self,
        layouter: &mut impl Layouter<F>,
        (fib0, fib1): &FibNums<F>,
    ) -> Result<FibNums<F>, Error> {
        layouter.group(
            || "fib",
            default_group_key!(),
            |layouter, group| {
                group.annotate_inputs([fib0.cell(), fib1.cell()])?;
                layouter.assign_region(
                    || "fib",
                    |mut region| {
                        self.config.selector.enable(&mut region, 0)?;
                        fib0.copy_advice(|| "fib0", &mut region, self.config.col_a, 0)?;
                        fib1.copy_advice(|| "fib1", &mut region, self.config.col_b, 0)?;

                        let fib2 = region.assign_advice(
                            || "fib2",
                            self.config.col_c,
                            0,
                            || fib0.value().copied() + fib1.value(),
                        )?;

                        group.annotate_outputs([fib1.cell(), fib2.cell()])?;
                        Ok((fib1.clone(), fib2))
                    },
                )
            },
        )
    }

    pub fn expose_outputs(
        &self,
        layouter: &mut impl Layouter<F>,
        cells: &[AssignedCell<F, F>],
        row: usize,
    ) -> Result<(), Error> {
        for (n, cell) in cells.iter().enumerate() {
            layouter.constrain_instance(cell.cell(), self.config.instance, row + n)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct FibonacciCircuit<F>(pub PhantomData<F>);

impl<F: Field> Circuit<F> for FibonacciCircuit<F> {
    type Config = FibonacciConfig;
    type FloorPlanner = SimpleFloorPlanner;
    type Params = ();

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FibonacciChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = FibonacciChip::construct(config);

        let mut fib = chip.assign_inputs(&mut layouter)?;

        for _ in 0..7 {
            fib = chip.step(&mut layouter, &fib)?;
        }

        chip.expose_outputs(&mut layouter, &[fib.0, fib.1], 2)?;
        Ok(())
    }
}
