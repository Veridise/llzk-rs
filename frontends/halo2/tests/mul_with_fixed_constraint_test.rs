use group::ff::Field;
use halo2_llzk_frontend::ir::generate::IRGenParamsBuilder;
use halo2_llzk_frontend::PicusParamsBuilder;
use halo2curves_070::bn256::Fr;
use midnight_halo2_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value};
use midnight_halo2_proofs::plonk::{
    Advice, Circuit, Column, ConstraintSystem, Error, Expression, Fixed, Instance, Selector,
};
use midnight_halo2_proofs::poly::Rotation;
use std::marker::PhantomData;

use halo2_llzk_frontend::{CircuitCallbacks, CircuitIO};

mod common;

const EXPECTED_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module Main)
(input in_0)
(output out_0)
(assert (= (* 1 (+ (* 21888242871839275222246405745257275088548364400416034343698204186575808495616 adv_0_0) (- adv_1_0))) 0))
(assert (= (* 1 (+ (* adv_0_0 adv_1_0) (- adv_2_0))) 0))
(assert (= (* 1 (+ (+ 21888242871839275222246405745257275088548364400416034343698204186575808495616 (- 2)) 3)) 0))
(assert (= adv_0_0 in_0))
(assert (= adv_2_0 out_0))
(assert (= 2 adv_3_0))
(assert (= 2 2))
(end-module)
";

#[test]
fn mul_with_fixed_constraint_circuit_picus() {
    common::setup();
    common::picus_test(
        MulWithFixedConstraintCircuit::<Fr>::default(),
        PicusParamsBuilder::new()
            .short_names()
            .no_optimize()
            .build(),
        IRGenParamsBuilder::new().build(),
        EXPECTED_PICUS,
        false,
    );
}

const EXPECTED_OPT_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module Main)
(input in_0)
(output out_0)
(assert (= (- in_0) adv_1_0))
(assert (= (* in_0 adv_1_0) out_0))
(assert (= 2 adv_3_0))
(end-module)
";

#[test]
fn mul_with_fixed_constraint_opt_circuit_picus() {
    common::setup();
    common::picus_test(
        MulWithFixedConstraintCircuit::<Fr>::default(),
        PicusParamsBuilder::new().short_names().build(),
        IRGenParamsBuilder::new().build(),
        EXPECTED_OPT_PICUS,
        true,
    );
}

#[derive(Debug, Clone)]
pub struct MulWithFixedConstraintConfig {
    pub col_fixed: Column<Fixed>,
    pub col_a: Column<Advice>,
    pub col_b: Column<Advice>,
    pub col_c: Column<Advice>,
    pub col_d: Column<Advice>,
    pub selector: Selector,
    pub instance: Column<Instance>,
}

#[derive(Debug, Clone)]
struct MulChip<F: Field> {
    config: MulWithFixedConstraintConfig,
    _marker: PhantomData<F>,
}

impl<F: Field> MulChip<F> {
    pub fn construct(config: MulWithFixedConstraintConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> MulWithFixedConstraintConfig {
        let col_fixed = meta.fixed_column();
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let col_d = meta.advice_column();
        let selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_constant(col_fixed);
        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(col_d);
        meta.enable_equality(instance);

        // computes c = -a^2
        meta.create_gate("mul", |meta| {
            //
            // col_fixed | col_a | col_b | col_c | selector
            //      f       a      b        c       s
            //
            let s = meta.query_selector(selector);
            let f = meta.query_fixed(col_fixed, Rotation::cur());
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());

            vec![s.clone() * (f * a.clone() - b.clone()), s * (a * b - c)]
        });

        meta.create_gate("equal -1", |meta| {
            let s = meta.query_selector(selector);
            let f = meta.query_fixed(col_fixed, Rotation::cur());
            let f2 = meta.query_fixed(col_fixed, Rotation::next());

            vec![s * ((f - f2) + Expression::Constant(F::ONE + F::ONE + F::ONE).into())]
        });

        MulWithFixedConstraintConfig {
            col_fixed,
            col_a,
            col_b,
            col_c,
            col_d,
            selector,
            instance,
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn assign_first_row(
        &self,
        mut layouter: impl Layouter<F>,
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

                let a_cell = region.assign_advice_from_instance(
                    || "a",
                    self.config.instance,
                    0,
                    self.config.col_a,
                    0,
                )?;

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

                region.assign_advice_from_constant(
                    || "const",
                    self.config.col_d,
                    0,
                    F::ONE + F::ONE,
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
pub struct MulWithFixedConstraintCircuit<F>(pub PhantomData<F>);

impl<F: Field> Circuit<F> for MulWithFixedConstraintCircuit<F> {
    type Config = MulWithFixedConstraintConfig;
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

        let prev_c = chip.assign_first_row(layouter.namespace(|| "first row"))?;

        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 1)?;
        Ok(())
    }
}

impl<F: Field> CircuitCallbacks<F> for MulWithFixedConstraintCircuit<F> {
    fn advice_io(_: &<Self as Circuit<F>>::Config) -> CircuitIO<Advice> {
        CircuitIO::empty()
    }

    fn instance_io(config: &<Self as Circuit<F>>::Config) -> CircuitIO<Instance> {
        CircuitIO::new(&[(config.instance, &[0])], &[(config.instance, &[1])])
    }
}
