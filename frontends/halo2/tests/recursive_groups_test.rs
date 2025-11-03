use group::ff::Field;
use halo2_llzk_frontend::ir::generate::IRGenParamsBuilder;
use halo2_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner};
use halo2_proofs::default_group_key;
use halo2_proofs::plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector};
use halo2_proofs::poly::Rotation;
use halo2curves_070::bn256::Fr;
use std::marker::PhantomData;

#[cfg(feature = "picus-backend")]
use halo2_llzk_frontend::PicusParamsBuilder;
use halo2_llzk_frontend::{CircuitIO, CircuitSynthesis};

mod common;

const N_INPUTS: usize = 4;

const EXPECTED_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module mul_many)
(input in_0)
(input in_1)
(output out_0)
(assert (= (* 1 (+ (* adv_0_4 adv_1_4) (- out_0))) 0))
(assert (= in_0 adv_0_4))
(assert (= in_1 adv_1_4))
(end-module)
(begin-module mul_many1)
(input in_0)
(input in_1)
(input in_2)
(output out_0)
(call [cout_0_0] mul_many [in_1 in_2])
(assert (= adv_2_4 cout_0_0))
(assert (= (* 1 (+ (* adv_0_5 adv_1_5) (- out_0))) 0))
(assert (= in_0 adv_0_5))
(assert (= adv_2_4 adv_1_5))
(end-module)
(begin-module mul_many2)
(input in_0)
(input in_1)
(input in_2)
(input in_3)
(output out_0)
(call [cout_0_0] mul_many1 [in_1 in_2 in_3])
(assert (= adv_2_5 cout_0_0))
(assert (= (* 1 (+ (* adv_0_6 adv_1_6) (- out_0))) 0))
(assert (= in_0 adv_0_6))
(assert (= adv_2_5 adv_1_6))
(end-module)
(begin-module Main)
(input in_0)
(input in_1)
(input in_2)
(input in_3)
(output out_0)
(call [cout_0_0] mul_many2 [adv_0_0 adv_0_1 adv_0_2 adv_0_3])
(assert (= adv_2_6 cout_0_0))
(assert (= adv_0_0 in_0))
(assert (= adv_0_1 in_1))
(assert (= adv_0_2 in_2))
(assert (= adv_0_3 in_3))
(assert (= adv_2_6 out_0))
(end-module)
";

#[cfg(feature = "picus-backend")]
#[test]
fn recursive_groups_circuit_picus() {
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

const EXPECTED_OPT_PICUS: &'static str = r"
(prime-number 21888242871839275222246405745257275088548364400416034343698204186575808495617)
(begin-module mul_many)
(input in_0)
(input in_1)
(output out_0)
(assert (= (* in_0 in_1) out_0))
(end-module)
(begin-module mul_many1)
(input in_0)
(input in_1)
(input in_2)
(output out_0)
(call [adv_2_4] mul_many [in_1 in_2])
(assert (= (* in_0 adv_2_4) out_0))
(end-module)
(begin-module mul_many2)
(input in_0)
(input in_1)
(input in_2)
(input in_3)
(output out_0)
(call [adv_2_5] mul_many1 [in_1 in_2 in_3])
(assert (= (* in_0 adv_2_5) out_0))
(end-module)
(begin-module Main)
(input in_0)
(input in_1)
(input in_2)
(input in_3)
(output out_0)
(call [out_0] mul_many2 [in_0 in_1 in_2 in_3])
(end-module)
";

#[cfg(feature = "picus-backend")]
#[test]
fn recursive_groups_opt_circuit_picus() {
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

            halo2_proofs::plonk::Constraints::with_selector(selector, vec![a * b - c])
        });

        MulConfig {
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
        let inputs: Vec<_> = (0..N_INPUTS).collect();
        CircuitIO::new(
            &[(config.instance, &inputs)],
            &[(config.instance, &[inputs.len()])],
        )
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
