use group::ff::Field;
use halo2_llzk_frontend::driver::Driver;
use halo2_llzk_frontend::ir::CmpOp;
use halo2_llzk_frontend::ir::generate::IRGenParamsBuilder;
use halo2_llzk_frontend::ir::stmt::IRStmt;
use halo2_proofs::circuit::{AssignedCell, Layouter, RegionIndex, SimpleFloorPlanner, Value};
use halo2_proofs::plonk::{
    Advice, Circuit, Column, ConstraintSystem, Error, Expression, Fixed, Instance, Selector,
};
use halo2_proofs::poly::Rotation;
use halo2curves_070::bn256::Fr;
use std::marker::PhantomData;

use halo2_llzk_frontend::{CircuitIO, CircuitSynthesis, ExpressionInRow, PicusParamsBuilder};

mod common;

const EXPECTED_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module Main)
(input in_0)
(output out_0)
(output out_1)
(output out_2)
(assert (= (* 1 (+ (* 21888242871839275222246405745257275088548364400416034343698204186575808495616 adv_0_0) (- adv_0_1))) 0))
(assert (= (* 1 (+ (* adv_0_0 adv_0_1) (- adv_1_0))) 0))
(assert (= (* 1 (+ (* 21888242871839275222246405745257275088548364400416034343698204186575808495616 adv_0_2) (- adv_0_3))) 0))
(assert (= (* 1 (+ (* adv_0_2 adv_0_3) (- adv_1_2))) 0))
(assert (= (* 1 (+ (* 21888242871839275222246405745257275088548364400416034343698204186575808495616 adv_0_4) (- adv_0_5))) 0))
(assert (= (* 1 (+ (* adv_0_4 adv_0_5) (- adv_1_4))) 0))
(assert (= adv_0_0 in_0))
(assert (= adv_0_2 in_0))
(assert (= adv_0_4 in_0))
(assert (= adv_1_0 out_0))
(assert (= adv_1_2 out_1))
(assert (= adv_1_4 out_2))
(assert (< adv_0_0 1000))
(assert (>= adv_0_1 1000))
(assert (< adv_0_2 1000))
(assert (>= adv_0_3 1000))
(assert (< adv_0_4 1000))
(assert (>= adv_0_5 1000))
(end-module)
";

const EXPECTED_OPT_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module Main)
(input in_0)
(output out_0)
(output out_1)
(output out_2)
(assert (= (- in_0) adv_0_1))
(assert (= (* in_0 adv_0_1) out_0))
(assert (= (- in_0) adv_0_3))
(assert (= (* in_0 adv_0_3) out_1))
(assert (= (- in_0) adv_0_5))
(assert (= (* in_0 adv_0_5) out_2))
(assert (< in_0 1000))
(assert (>= adv_0_1 1000))
(assert (< in_0 1000))
(assert (>= adv_0_3 1000))
(assert (< in_0 1000))
(assert (>= adv_0_5 1000))
(end-module)
";

fn ir_to_inject<'e>() -> Vec<(RegionIndex, IRStmt<ExpressionInRow<'e, Fr>>)> {
    let mut cs = ConstraintSystem::default();
    let config = <MulCircuit<Fr> as Circuit<Fr>>::configure(&mut cs);
    let a = config.col_a.cur();
    let hundrend = Expression::Constant(Fr::from(1000));
    let stmts = [
        IRStmt::constraint(
            CmpOp::Lt,
            ExpressionInRow::new(0, a.clone()),
            ExpressionInRow::new(0, hundrend.clone()),
        ),
        IRStmt::constraint(
            CmpOp::Ge,
            ExpressionInRow::new(1, a),
            ExpressionInRow::new(1, hundrend),
        ),
    ];

    let mut injected = vec![];
    for row in 0..6 {
        let index = RegionIndex::from(row / 2);
        let offset = row % 2;

        let payload = (index, stmts[offset].clone());
        log::debug!("payload = {payload:?}");
        injected.push(payload);
    }
    injected
}

#[test]
fn mul_injected_circuit_picus() {
    common::setup();
    let circuit = MulCircuit::<Fr>::default();
    let mut driver = Driver::default();
    let resolved = {
        let syn = driver.synthesize(&circuit).unwrap();

        let mut unresolved = driver
            .generate_ir(&syn, IRGenParamsBuilder::new().build())
            .unwrap();
        let ir = ir_to_inject();
        unresolved.inject_ir(ir, &syn).unwrap();
        let (status, errs) = unresolved.validate();
        if status.is_err() {
            for err in errs {
                log::error!("{err}");
            }
            panic!("Test failed due to validation errors");
        }
        let resolved = unresolved.resolve().unwrap();
        let (status, errs) = resolved.validate();
        if status.is_err() {
            for err in errs {
                log::error!("{err}");
            }
            panic!("Test failed due to validation errors");
        }
        resolved
    };

    common::check_picus(
        &driver,
        &resolved,
        PicusParamsBuilder::new()
            .short_names()
            .no_optimize()
            .build(),
        EXPECTED_PICUS,
    );
}

#[test]
fn mul_injected_opt_circuit_picus() {
    common::setup();
    {
        let circuit = MulCircuit::<Fr>::default();
        let mut driver = Driver::default();
        let mut resolved = {
            let syn = driver.synthesize(&circuit).unwrap();

            let mut unresolved = driver
                .generate_ir(&syn, IRGenParamsBuilder::new().build())
                .unwrap();
            let ir = ir_to_inject();
            unresolved.inject_ir(ir, &syn).unwrap();
            let (status, errs) = unresolved.validate();
            if status.is_err() {
                for err in errs {
                    log::error!("{err}");
                }
                panic!("Test failed due to validation errors");
            }
            let resolved = unresolved.resolve().unwrap();
            let (status, errs) = resolved.validate();
            if status.is_err() {
                for err in errs {
                    log::error!("{err}");
                }
                panic!("Test failed due to validation errors");
            }
            resolved
        };
        resolved.constant_fold().unwrap();
        let (status, errs) = resolved.validate();
        if status.is_err() {
            for err in errs {
                log::error!("{err}");
            }
            panic!("Test failed due to validation errors");
        }
        resolved.canonicalize();
        let (status, errs) = resolved.validate();
        if status.is_err() {
            for err in errs {
                log::error!("{err}");
            }
            panic!("Test failed due to validation errors");
        }
        common::check_picus(
            &driver,
            &resolved,
            PicusParamsBuilder::new().short_names().build(),
            EXPECTED_OPT_PICUS,
        );
    };
}

#[derive(Debug, Clone)]
pub struct MulConfig {
    pub col_fixed: Column<Fixed>,
    pub col_a: Column<Advice>,
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
        let col_c = meta.advice_column();
        let selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_constant(col_fixed);
        meta.enable_equality(col_a);
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
            let b = meta.query_advice(col_a, Rotation::next());
            let c = meta.query_advice(col_c, Rotation::cur());

            halo2_proofs::plonk::Constraints::with_selector(
                selector,
                vec![f * a.clone() - b.clone(), a * b - c],
            )
        });

        MulConfig {
            col_fixed,
            col_a,
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
                    self.config.col_a,
                    1,
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
        let prev_c = chip.assign_first_row(layouter.namespace(|| "first row"))?;
        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 2)?;
        let prev_c = chip.assign_first_row(layouter.namespace(|| "first row"))?;
        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 3)?;
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
        CircuitIO::new(&[(config.instance, &[0])], &[(config.instance, &[1, 2, 3])])
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
