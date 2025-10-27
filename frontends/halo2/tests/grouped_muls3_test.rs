//! This test checks that calling the same group multiple types creates only one module and two
//! calls.

use group::ff::Field;
use halo2_llzk_frontend::ir::generate::IRGenParamsBuilder;
use halo2_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value};
use halo2_proofs::default_group_key;
use halo2_proofs::plonk::{
    Advice, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector,
};
use halo2_proofs::poly::Rotation;
use halo2curves_070::bn256::Fr;
use std::marker::PhantomData;

use halo2_llzk_frontend::{CircuitIO, CircuitSynthesis, PicusParamsBuilder};

mod common;

const EXPECTED_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module test_group)
(input in_0)
(output out_0)
(assert (= (* 1 (+ (* 21888242871839275222246405745257275088548364400416034343698204186575808495616 adv_0_1) (- adv_1_1))) 0))
(assert (= (* 1 (+ (* adv_0_1 adv_1_1) (- out_0))) 0))
(assert (= in_0 adv_0_1))
(end-module)
(begin-module Main)
(input in_0)
(output out_0)
(call [cout_0_0] test_group [adv_0_0])
(assert (= adv_2_1 cout_0_0))
(call [cout_1_0] test_group [adv_2_1])
(assert (= adv_2_2 cout_1_0))
(assert (= adv_0_0 in_0))
(assert (= adv_2_2 adv_2_3))
(assert (= adv_2_3 out_0))
(end-module)
";

const EXPECTED_OPT_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module test_group)
(input in_0)
(output out_0)
(assert (= (- in_0) adv_1_1))
(assert (= (* in_0 adv_1_1) out_0))
(end-module)
(begin-module Main)
(input in_0)
(output out_0)
(call [adv_2_1] test_group [in_0])
(call [out_0] test_group [adv_2_1])
(end-module)
";

#[test]
fn groped_mul3_circuit_picus() {
    common::setup();
    common::picus_test(
        MulCircuit::<Fr>::default(),
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
fn groped_mul3_opt_circuit_picus() {
    common::setup();
    common::picus_test(
        MulCircuit::<Fr>::default(),
        PicusParamsBuilder::new().short_names().build(),
        IRGenParamsBuilder::new().build(),
        EXPECTED_OPT_PICUS,
        true,
    );
}

#[derive(Debug, Clone)]
pub struct MulConfig {
    pub col_fixed: Column<Fixed>,
    pub col_a: Column<Advice>,
    pub col_b: Column<Advice>,
    pub col_c: Column<Advice>,
    pub selector: Selector,
    pub instance: Column<Instance>,
}

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
        let col_fixed = meta.fixed_column();
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_constant(col_fixed);
        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        // computes c = -a^2
        meta.create_gate("mul", |meta| {
            //
            // col_fixed | col_a | col_b | col_c | selector
            //      f       a      b        c       s
            //
            let f = meta.query_fixed(col_fixed, Rotation::cur());
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());

            halo2_proofs::plonk::Constraints::with_selector(
                selector,
                vec![f * a.clone() - b.clone(), a * b - c],
            )
        });

        MulConfig {
            col_fixed,
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
        layouter: &mut impl Layouter<F>,
        input: &AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "first row",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;

                let fixed_cell = region.assign_fixed(
                    || "-1",
                    self.config.col_fixed,
                    0,
                    || -> Value<F> { Value::known(-F::ONE) },
                )?;

                let a_cell = region.assign_advice(
                    || "a",
                    self.config.col_a,
                    0,
                    || input.value().copied(),
                )?;
                region.constrain_equal(input.cell(), a_cell.cell())?;

                let b_cell = region.assign_advice(
                    || "-1 * a",
                    self.config.col_b,
                    0,
                    || a_cell.value().copied() * fixed_cell.value(),
                )?;

                let c_cell = region.assign_advice(
                    || "a * b",
                    self.config.col_c,
                    0,
                    || a_cell.value().copied() * b_cell.value(),
                )?;

                Ok(c_cell)
            },
        )
    }

    pub fn assign_a(&self, layouter: &mut impl Layouter<F>) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "set a",
            |mut region| {
                region.assign_advice_from_instance(
                    || "a",
                    self.config.instance,
                    0,
                    self.config.col_a,
                    0,
                )
            },
        )
    }

    pub fn assign_c(
        &self,
        layouter: &mut impl Layouter<F>,
        cell: &AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "set c",
            |mut region| {
                let c =
                    region.assign_advice(|| "c", self.config.col_c, 0, || cell.value().copied())?;
                region.constrain_equal(cell.cell(), c.cell())?;
                Ok(c)
            },
        )
    }

    pub fn expose_public(
        &self,
        layouter: &mut impl Layouter<F>,
        cell: &AssignedCell<F, F>,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, row)
    }

    pub fn call_group(
        &self,
        layouter: &mut impl Layouter<F>,
        input: &AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.group(
            || "test group",
            // Defined here to get always the same key.
            default_group_key!(),
            |layouter, group| {
                group.annotate_input(input.cell())?;
                let prev_c = self.assign_first_row(layouter, input)?;
                group.annotate_output(prev_c.cell())?;
                Ok(prev_c)
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
        let a = chip.assign_a(&mut layouter)?;
        let c = chip.call_group(&mut layouter, &a)?;
        let c = chip.call_group(&mut layouter, &c)?;
        let pub_c = chip.assign_c(&mut layouter, &c)?;
        chip.expose_public(&mut layouter, &pub_c, 1)?;

        Ok(())
    }
}

impl<F: Field> CircuitSynthesis<F> for MulCircuit<F> {
    type Circuit = Self;
    type Config = MulConfig;

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
        CircuitIO::new(&[(config.instance, &[0])], &[(config.instance, &[1])])
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
