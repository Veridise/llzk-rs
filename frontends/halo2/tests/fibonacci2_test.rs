use group::ff::Field;
use halo2_llzk_frontend::ir::generate::IRGenParamsBuilder;
use halo2_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner};
use halo2_proofs::default_group_key;
use halo2_proofs::plonk::{
    Advice, Circuit, Column, ConstraintSystem, Constraints, Error, Instance, Selector,
};
use halo2_proofs::poly::Rotation;
use std::marker::PhantomData;

use halo2_llzk_frontend::{CircuitIO, CircuitSynthesis, PicusParamsBuilder};
use halo2curves_070::bn256::Fr;

mod common;

const EXPECTED_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module fib)
(input in_0)
(input in_1)
(output out_0)
(output out_1)
(assert (= (* 1 (+ (+ adv_0_1 adv_1_1) (- out_1))) 0))
(assert (= adv_0_1 in_0))
(assert (= adv_1_1 out_0))
(assert (= in_1 out_0))
(end-module)
(begin-module Main)
(input in_0)
(input in_1)
(output out_0)
(output out_1)
(call [cout_0_0 cout_0_1] fib [adv_0_0 adv_1_0])
(assert (= adv_1_0 cout_0_0))
(assert (= adv_2_1 cout_0_1))
(call [cout_1_0 cout_1_1] fib [adv_1_0 adv_2_1])
(assert (= adv_2_1 cout_1_0))
(assert (= adv_2_2 cout_1_1))
(call [cout_2_0 cout_2_1] fib [adv_2_1 adv_2_2])
(assert (= adv_2_2 cout_2_0))
(assert (= adv_2_3 cout_2_1))
(call [cout_3_0 cout_3_1] fib [adv_2_2 adv_2_3])
(assert (= adv_2_3 cout_3_0))
(assert (= adv_2_4 cout_3_1))
(call [cout_4_0 cout_4_1] fib [adv_2_3 adv_2_4])
(assert (= adv_2_4 cout_4_0))
(assert (= adv_2_5 cout_4_1))
(call [cout_5_0 cout_5_1] fib [adv_2_4 adv_2_5])
(assert (= adv_2_5 cout_5_0))
(assert (= adv_2_6 cout_5_1))
(call [cout_6_0 cout_6_1] fib [adv_2_5 adv_2_6])
(assert (= adv_2_6 cout_6_0))
(assert (= adv_2_7 cout_6_1))
(assert (= (* 1 (+ (+ adv_0_0 adv_1_0) (- adv_2_0))) 0))
(assert (= adv_0_0 in_0))
(assert (= adv_1_0 in_1))
(assert (= adv_2_6 out_0))
(assert (= adv_2_7 out_1))
(end-module)
";

const EXPECTED_OPT_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module fib)
(input in_0)
(input in_1)
(output out_0)
(output out_1)
(assert (= (+ in_0 in_1) out_1))
(assert (= in_1 out_0))
(assert (= in_1 out_0))
(end-module)
(begin-module Main)
(input in_0)
(input in_1)
(output out_0)
(output out_1)
(call [in_1 adv_2_1] fib [in_0 in_1])
(call [adv_2_1 adv_2_2] fib [in_1 adv_2_1])
(call [adv_2_2 adv_2_3] fib [adv_2_1 adv_2_2])
(call [adv_2_3 adv_2_4] fib [adv_2_2 adv_2_3])
(call [adv_2_4 adv_2_5] fib [adv_2_3 adv_2_4])
(call [adv_2_5 out_0] fib [adv_2_4 adv_2_5])
(call [out_0 out_1] fib [adv_2_5 out_0])
(assert (= (+ in_0 in_1) adv_2_0))
(end-module)
";

#[test]
fn fibonacci2_circuit_picus() {
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
fn fibonacci2_opt_circuit_picus() {
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

type FibNums<F> = (AssignedCell<F, F>, AssignedCell<F, F>);

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

impl<F: Field> CircuitSynthesis<F> for FibonacciCircuit<F> {
    type Circuit = Self;
    type Config = FibonacciConfig;
    type CS = ConstraintSystem<F>;
    type Error = halo2_proofs::plonk::Error;

    fn circuit(&self) -> &Self::Circuit {
        self
    }

    fn configure(cs: &mut Self::CS) -> Self::Config {
        <Self as Circuit<F>>::configure(cs)
    }

    fn advice_io(_: &<Self as Circuit<F>>::Config) -> anyhow::Result<CircuitIO<Advice>> {
        Ok(CircuitIO::empty())
    }

    fn instance_io(config: &<Self as Circuit<F>>::Config) -> anyhow::Result<CircuitIO<Instance>> {
        CircuitIO::new(&[(config.instance, &[0, 1])], &[(config.instance, &[2, 3])])
    }

    fn synthesize(
        circuit: &Self::Circuit,
        config: Self::Config,
        synthesizer: &mut halo2_llzk_frontend::Synthesizer<F>,
        cs: &Self::CS,
    ) -> Result<(), Self::Error> {
        common::SynthesizerAssignment::synthesize(circuit, config, synthesizer, cs)
    }
}
