use std::{borrow::Cow, cell::RefCell};

use ff::{Field, PrimeField};
#[cfg(feature = "picus-backend")]
use halo2_llzk_frontend::PicusParams;
use halo2_llzk_frontend::{
    CircuitSynthesis, Synthesizer,
    driver::Driver,
    expressions::{EvalExpression, EvaluableExpr, ExprBuilder, ExpressionInfo, ExpressionTypes},
    info_traits::{
        ChallengeInfo, ConstraintSystemInfo, CreateQuery, GateInfo, GroupInfo, QueryInfo,
        SelectorInfo,
    },
    ir::{
        ResolvedIRCircuit,
        generate::{IRGenParams, IRGenParamsBuilder},
        stmt::IRStmt,
    },
    lookups::{Lookup, callbacks::LookupCallbacks, table::LookupTableGenerator},
    temps::{ExprOrTemp, Temps},
};
use halo2_midnight_integration::plonk::{_Expression, ConstraintSystem};
use halo2_proofs::plonk::{Any, Challenge, Column, FloorPlanner};
use halo2_proofs::{
    circuit::{
        Value,
        groups::{GroupKey, GroupKeyInstance, RegionsGroup},
    },
    plonk::{Advice, Assignment, Circuit, Error, Expression, Fixed, Instance, Selector},
    utils::rational::Rational,
};
use log::LevelFilter;
use simplelog::{Config, TestLogger};

macro_rules! basic_picus_test {
    ($name:ident, $circuit:expr, $expected:expr, $expected_opt:expr) => {
        mod $name {
            use super::*;
            #[cfg(feature = "picus-backend")]
            #[test]
            fn picus() {
                common::setup();
                common::picus_test(
                    $circuit,
                    common::picus_params(),
                    common::no_ir_gen_params(),
                    $expected,
                    false,
                );
            }

            #[cfg(feature = "picus-backend")]
            #[test]
            fn opt_picus() {
                common::setup();
                common::picus_test(
                    $circuit,
                    common::opt_picus_params(),
                    common::no_ir_gen_params(),
                    $expected_opt,
                    true,
                );
            }
        }
    };
}

pub(crate) use basic_picus_test;

#[cfg(feature = "picus-backend")]
pub fn picus_params() -> PicusParams {
    halo2_llzk_frontend::PicusParamsBuilder::new()
        .short_names()
        .no_optimize()
        .build()
}

#[cfg(feature = "picus-backend")]
pub fn opt_picus_params() -> PicusParams {
    halo2_llzk_frontend::PicusParamsBuilder::new()
        .short_names()
        .build()
}

pub fn no_ir_gen_params<F: Field>() -> IRGenParams<'static, 'static, F, _Expression<F>> {
    IRGenParamsBuilder::new().build()
}

pub fn setup() {
    let _ = TestLogger::init(LevelFilter::Debug, Config::default());
}

/// We run the synthesis separately to test that the lifetimes of the values
/// can be untied to the CircuitSynthesis struct. But also if we want to add LLZK tests
/// this makes sure to test the retargeability of the driver.
pub fn synthesize_and_generate_ir<'drv, F, C>(
    driver: &'drv mut Driver,
    circuit: C,
    params: IRGenParams<F, _Expression<F>>,
) -> ResolvedIRCircuit
where
    F: PrimeField,
    C: CircuitSynthesis<F>,
    C: CircuitSynthesis<F, CS = ConstraintSystem<F>>,
{
    let syn = driver.synthesize(&circuit).unwrap();
    let unresolved = driver.generate_ir(&syn, params).unwrap();
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
}

#[cfg(feature = "picus-backend")]
#[allow(dead_code)]
pub fn picus_test<F, C>(
    circuit: C,
    params: PicusParams,
    ir_params: IRGenParams<F, _Expression<F>>,
    expected: impl AsRef<str>,
    canonicalize: bool,
) where
    F: PrimeField,
    C: CircuitSynthesis<F, CS = ConstraintSystem<F>>,
{
    let mut driver = Driver::default();
    let mut resolved = synthesize_and_generate_ir(&mut driver, circuit, ir_params);
    if canonicalize {
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
    }
    check_picus(&driver, &resolved, params, expected);
}

#[cfg(feature = "picus-backend")]
pub fn check_picus(
    driver: &Driver,
    circuit: &ResolvedIRCircuit,
    params: PicusParams,
    expected: impl AsRef<str>,
) {
    let output = clean_string(
        &driver
            .picus(&circuit, params)
            .unwrap()
            .display()
            .to_string(),
    );
    let expected = clean_string(expected.as_ref());
    similar_asserts::assert_eq!(expected, output);
}

fn clean_string(s: &str) -> String {
    let mut r = String::with_capacity(s.len());
    for line in s.lines() {
        let line = line.trim();
        if line.starts_with(";") || line.is_empty() {
            continue;
        }
        let line = match line.find(';') {
            Some(idx) => &line[..idx],
            None => line,
        }
        .trim();

        r.extend(line.chars());
        r.extend("\n".chars());
    }
    r
}

#[allow(dead_code)]

struct ValueStealer<T> {
    data: RefCell<Option<T>>,
}

impl<T: Clone> ValueStealer<T> {
    fn new() -> Self {
        Self {
            data: RefCell::new(None),
        }
    }

    fn steal(&self, value: Value<T>) -> Option<T> {
        value.map(|t| self.data.replace(Some(t)));
        self.data.replace(None)
    }
}

/// Transforms a [`Value`] into an [`Option`], returning None if the value is unknown.
pub fn steal<T: Clone>(value: &Value<T>) -> Option<T> {
    let stealer = ValueStealer::<T>::new();
    stealer.steal(value.clone())
}

fn to_plonk_error<E>(error: E) -> Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    Error::Transcript(std::io::Error::other(error))
}

macro_rules! synthesis_impl {
    ($name:ident, $circuit:ty, $inputs:expr, $outputs:expr) => {
        #[derive(Default)]
        struct $name($circuit);

        impl halo2_llzk_frontend::CircuitSynthesis<halo2curves::bn256::Fr> for $name {
            type Circuit = $circuit;
            type Config =
                <$circuit as halo2_proofs::plonk::Circuit<halo2curves::bn256::Fr>>::Config;

            type CS = halo2_midnight_integration::plonk::ConstraintSystem<halo2curves::bn256::Fr>;

            type Error = halo2_proofs::plonk::Error;

            fn circuit(&self) -> &Self::Circuit {
                &self.0
            }
            fn configure(cs: &mut Self::CS) -> Self::Config {
                <$circuit as halo2_proofs::plonk::Circuit<halo2curves::bn256::Fr>>::configure(
                    cs.inner_mut(),
                )
            }

            fn advice_io(_: &Self::Config) -> anyhow::Result<halo2_llzk_frontend::AdviceIO> {
                Ok(halo2_llzk_frontend::CircuitIO::empty())
            }
            fn instance_io(
                config: &Self::Config,
            ) -> anyhow::Result<halo2_llzk_frontend::InstanceIO> {
                halo2_llzk_frontend::CircuitIO::new::<
                    halo2_midnight_integration::plonk::_Column<
                        halo2_midnight_integration::plonk::_Instance,
                    >,
                >(
                    &[(config.instance.into(), &$inputs)],
                    &[(config.instance.into(), &$outputs)],
                )
            }
            fn synthesize(
                circuit: &Self::Circuit,
                config: Self::Config,
                synthesizer: &mut halo2_llzk_frontend::Synthesizer<halo2curves::bn256::Fr>,
                cs: &Self::CS,
            ) -> Result<(), Self::Error> {
                halo2_midnight_integration::synthesizer::SynthesizerAssignment::synthesize(
                    circuit,
                    config,
                    synthesizer,
                    cs.inner(),
                )
            }
        }
    };
}

pub(crate) use synthesis_impl;
