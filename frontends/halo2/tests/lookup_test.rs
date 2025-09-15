use group::ff::Field;
use halo2curves_070::bn256::Fr;
use midnight_halo2_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value};
use midnight_halo2_proofs::plonk::{
    Advice, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector, TableColumn,
};
use midnight_halo2_proofs::poly::Rotation;
use std::marker::PhantomData;

use halo2_llzk_frontend::{
    lookups::callbacks::LookupCallbacks, CircuitCallbacks, CircuitIO, PicusParamsBuilder,
};

mod common;

const EXPECTED_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module Main)
(input in_0)
(output out_0)
(assert (= (* 1 (+ (* adv_1_0 adv_0_0) (- adv_2_0))) 0))
(assert (= (* 1 (+ (* adv_0_0 adv_2_0) (- adv_3_0))) 0))
(assert (= adv_0_0 in_0))
(assert (= adv_3_0 out_0))
(end-module)
";

#[test]
fn lookup_circuit_picus() {
    common::setup();
    common::picus_test(
        LookupCircuit::<Fr>::default(),
        PicusParamsBuilder::new::<Fr>()
            .short_names()
            .no_optimize()
            .build(),
        EXPECTED_PICUS,
    );
}

#[derive(Debug, Clone)]
pub struct LookupConfig {
    #[allow(dead_code)]
    pub col_fixed: Column<Fixed>,
    pub lookup_column: TableColumn,
    pub col_f: Column<Advice>,
    pub col_a: Column<Advice>,
    pub col_b: Column<Advice>,
    pub col_c: Column<Advice>,
    pub selector: Selector,
    pub instance: Column<Instance>,
}

#[derive(Debug, Clone)]
struct LookupChip<F: Field> {
    config: LookupConfig,
    _marker: PhantomData<F>,
}

impl<F: Field> LookupChip<F> {
    pub fn construct(config: LookupConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> LookupConfig {
        let col_fixed = meta.fixed_column();
        let col_a = meta.advice_column();
        let col_f = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let selector = meta.complex_selector();
        let instance = meta.instance_column();

        meta.enable_constant(col_fixed);
        meta.enable_equality(col_a);
        meta.enable_equality(col_f);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        let lookup_column = meta.lookup_table_column();

        meta.lookup("lookup test", |meta| {
            let s = meta.query_selector(selector);
            let f = meta.query_advice(col_f, Rotation::cur());

            vec![(s * f, lookup_column)]
        });

        // computes c = -a^2
        meta.create_gate("mul", |meta| {
            //
            // col_f | col_a | col_b | col_c | selector
            //   f       a      b        c       s
            //
            let s = meta.query_selector(selector);
            let f = meta.query_advice(col_f, Rotation::cur());
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());

            vec![s.clone() * (f * a.clone() - b.clone()), s * (a * b - c)]
        });

        LookupConfig {
            col_fixed,
            lookup_column,
            col_a,
            col_f,
            col_b,
            col_c,
            selector,
            instance,
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn assign_table(&self, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "table",
            |mut table| {
                table.assign_cell(
                    || "lookup col",
                    self.config.lookup_column,
                    0,
                    || -> Value<F> { Value::known(-F::ONE) },
                )
            },
        )
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

                let fixed_cell = region.assign_advice(
                    || "-1",
                    self.config.col_f,
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
pub struct LookupCircuit<F>(pub PhantomData<F>);

impl<F: Field> Circuit<F> for LookupCircuit<F> {
    type Config = LookupConfig;
    type FloorPlanner = SimpleFloorPlanner;
    type Params = ();

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        LookupChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = LookupChip::construct(config);
        chip.assign_table(layouter.namespace(|| "table"))?;
        let prev_c = chip.assign_first_row(layouter.namespace(|| "first row"))?;

        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 1)?;
        Ok(())
    }
}

impl<F: Field> CircuitCallbacks<F, Self> for LookupCircuit<F> {
    fn advice_io(_: &<Self as Circuit<F>>::Config) -> CircuitIO<Advice> {
        CircuitIO::empty()
    }

    fn instance_io(config: &<Self as Circuit<F>>::Config) -> CircuitIO<Instance> {
        CircuitIO::new(&[(config.instance, &[0])], &[(config.instance, &[1])])
    }

    fn lookup_callbacks() -> Option<Box<dyn LookupCallbacks<F>>> {
        Some(Box::new(common::LookupCallbackHandler))
    }
}
