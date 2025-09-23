use group::ff::Field;
use halo2_llzk_frontend::ir::generate::IRGenParamsBuilder;
use halo2_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner};
use halo2_proofs::plonk::{
    Advice, Circuit, Column, ConstraintSystem, Constraints, Error, Instance, Selector,
};
use halo2_proofs::poly::Rotation;
use std::marker::PhantomData;

use halo2_llzk_frontend::{CircuitCallbacks, CircuitIO, PicusParamsBuilder};
use halo2curves_070::bn256::Fr;

mod common;

const EXPECTED_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module Main)
(input in_0)
(input in_1)
(output out_0)
(assert (= (* 1 (+ (+ adv_0_0 adv_1_0) (- adv_2_0))) 0))
(assert (= (* 1 (+ (+ adv_0_1 adv_1_1) (- adv_2_1))) 0))
(assert (= (* 1 (+ (+ adv_0_2 adv_1_2) (- adv_2_2))) 0))
(assert (= (* 1 (+ (+ adv_0_3 adv_1_3) (- adv_2_3))) 0))
(assert (= (* 1 (+ (+ adv_0_4 adv_1_4) (- adv_2_4))) 0))
(assert (= (* 1 (+ (+ adv_0_5 adv_1_5) (- adv_2_5))) 0))
(assert (= (* 1 (+ (+ adv_0_6 adv_1_6) (- adv_2_6))) 0))
(assert (= (* 1 (+ (+ adv_0_7 adv_1_7) (- adv_2_7))) 0))
(assert (= adv_0_0 in_0))
(assert (= adv_0_1 adv_1_0))
(assert (= adv_0_2 adv_2_0))
(assert (= adv_0_3 adv_2_1))
(assert (= adv_0_4 adv_2_2))
(assert (= adv_0_5 adv_2_3))
(assert (= adv_0_6 adv_2_4))
(assert (= adv_0_7 adv_2_5))
(assert (= adv_1_0 in_1))
(assert (= adv_1_1 adv_2_0))
(assert (= adv_1_2 adv_2_1))
(assert (= adv_1_3 adv_2_2))
(assert (= adv_1_4 adv_2_3))
(assert (= adv_1_5 adv_2_4))
(assert (= adv_1_6 adv_2_5))
(assert (= adv_1_7 adv_2_6))
(assert (= adv_2_7 out_0))
(end-module)
";

const EXPECTED_OPT_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module Main)
(input in_0)
(input in_1)
(output out_0)
(assert (= (+ in_0 in_1) adv_0_2))
(assert (= (+ in_1 adv_0_2) adv_0_3))
(assert (= (+ adv_0_2 adv_0_3) adv_0_4))
(assert (= (+ adv_0_3 adv_0_4) adv_0_5))
(assert (= (+ adv_0_4 adv_0_5) adv_0_6))
(assert (= (+ adv_0_5 adv_0_6) adv_0_7))
(assert (= (+ adv_0_6 adv_0_7) adv_1_7))
(assert (= (+ adv_0_7 adv_1_7) out_0))
(end-module)
";

#[test]
fn fibonacci_circuit_picus() {
    common::setup();
    common::picus_test(
        FibonacciCircuit::<Fr>::default(),
        PicusParamsBuilder::new()
            .short_names()
            .no_optimize()
            .build(),
        IRGenParamsBuilder::new().build(),
        EXPECTED_PICUS,
        false,
    );
}

#[test]
fn fibonacci_opt_circuit_picus() {
    common::setup();
    common::picus_test(
        FibonacciCircuit::<Fr>::default(),
        PicusParamsBuilder::new().short_names().build(),
        IRGenParamsBuilder::new().build(),
        EXPECTED_OPT_PICUS,
        true,
    );
}

#[derive(Debug, Clone)]
pub struct FibonacciConfig {
    pub col_a: Column<Advice>,
    pub col_b: Column<Advice>,
    pub col_c: Column<Advice>,
    pub selector: Selector,
    pub instance: Column<Instance>,
}

#[derive(Debug, Clone)]
struct FibonacciChip<F: Field> {
    config: FibonacciConfig,
    _marker: PhantomData<F>,
}

impl<F: Field> FibonacciChip<F> {
    pub fn construct(config: FibonacciConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> FibonacciConfig {
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        meta.create_gate("add", |meta| {
            //
            // col_a | col_b | col_c | selector
            //   a      b        c       s
            //
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());

            Constraints::with_selector(selector, vec![a + b - c])
        });

        FibonacciConfig {
            col_a,
            col_b,
            col_c,
            selector,
            instance,
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn assign_first_row(
        &self,
        mut layouter: impl Layouter<F>,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        layouter.assign_region(
            || "first row",
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

                let c_cell = region.assign_advice(
                    || "a + b",
                    self.config.col_c,
                    0,
                    || a_cell.value().copied() + b_cell.value(),
                )?;

                Ok((a_cell, b_cell, c_cell))
            },
        )
    }

    pub fn assign_row(
        &self,
        mut layouter: impl Layouter<F>,
        prev_b: &AssignedCell<F, F>,
        prev_c: &AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "next row",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;

                // Copy the value from b & c in previous row to a & b in current row
                prev_b.copy_advice(|| "a", &mut region, self.config.col_a, 0)?;
                prev_c.copy_advice(|| "b", &mut region, self.config.col_b, 0)?;

                let c_cell = region.assign_advice(
                    || "c",
                    self.config.col_c,
                    0,
                    || prev_b.value().copied() + prev_c.value(),
                )?;

                Ok(c_cell)
            },
        )
    }

    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        cell: &AssignedCell<F, F>,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, row)
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

        let (_, mut prev_b, mut prev_c) =
            chip.assign_first_row(layouter.namespace(|| "first row"))?;

        for _i in 3..10 {
            let c_cell = chip.assign_row(layouter.namespace(|| "next row"), &prev_b, &prev_c)?;
            prev_b = prev_c;
            prev_c = c_cell;
        }

        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 2)?;
        Ok(())
    }
}

impl<F: Field> CircuitCallbacks<F> for FibonacciCircuit<F> {
    fn advice_io(_: &<Self as Circuit<F>>::Config) -> CircuitIO<Advice> {
        CircuitIO::empty()
    }

    fn instance_io(config: &<Self as Circuit<F>>::Config) -> CircuitIO<Instance> {
        CircuitIO::new(&[(config.instance, &[0, 1])], &[(config.instance, &[2])])
    }
}
