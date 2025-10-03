use group::ff::Field;
use halo2_llzk_frontend::ir::generate::IRGenParamsBuilder;
use halo2_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value};
use halo2_proofs::plonk::{
    Advice, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector,
};
use halo2_proofs::poly::Rotation;
use halo2curves_070::bn256::Fr;
use llzk::prelude::LlzkContext;
use std::marker::PhantomData;

use halo2_llzk_frontend::{CircuitCallbacks, CircuitIO, LlzkParamsBuilder, PicusParamsBuilder};

mod common;

const EXPECTED_PICUS: &'static str = include_str!("expected/picus/mul_test.picus");
const EXPECTED_OPT_PICUS: &'static str = include_str!("expected/picus/mul_test_opt.picus");
const EXPECTED_LLZK: &'static str = include_str!("expected/llzk/mul_test.mlir");
const EXPECTED_OPT_LLZK: &'static str = include_str!("expected/llzk/mul_test_opt.mlir");

#[test]
fn mul_circuit_picus() {
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
fn mul_opt_circuit_picus() {
    common::setup();
    common::picus_test(
        MulCircuit::<Fr>::default(),
        PicusParamsBuilder::new().short_names().build(),
        IRGenParamsBuilder::new().build(),
        EXPECTED_OPT_PICUS,
        true,
    );
}

#[test]
fn mul_circuit_llzk() {
    common::setup();
    let context = LlzkContext::new();
    common::llzk_test(
        MulCircuit::<Fr>::default(),
        LlzkParamsBuilder::new(&context).no_optimize().build(),
        IRGenParamsBuilder::new().build(),
        EXPECTED_LLZK,
        false,
    );
}

#[test]
fn mul_opt_circuit_llzk() {
    common::setup();
    let context = LlzkContext::new();
    common::llzk_test(
        MulCircuit::<Fr>::default(),
        LlzkParamsBuilder::new(&context).build(),
        IRGenParamsBuilder::new().build(),
        EXPECTED_OPT_LLZK,
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

        let prev_c = chip.assign_first_row(layouter.namespace(|| "first row"))?;

        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 1)?;
        Ok(())
    }
}

impl<F: Field> CircuitCallbacks<F> for MulCircuit<F> {
    fn advice_io(_: &<Self as Circuit<F>>::Config) -> CircuitIO<Advice> {
        CircuitIO::empty()
    }

    fn instance_io(config: &<Self as Circuit<F>>::Config) -> CircuitIO<Instance> {
        CircuitIO::new(&[(config.instance, &[0])], &[(config.instance, &[1])])
    }
}
