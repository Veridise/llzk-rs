use std::borrow::Cow;

use ff::{Field, PrimeField};
use halo2_llzk_frontend::{
    driver::Driver,
    ir::{generate::IRGenParams, stmt::IRStmt, ResolvedIRCircuit},
    lookups::{
        callbacks::{LookupCallbacks, LookupTableGenerator},
        Lookup,
    },
    temps::{ExprOrTemp, Temps},
    CircuitCallbacks, PicusParams,
};
use halo2_proofs::plonk::Expression;
use log::LevelFilter;
use simplelog::{Config, TestLogger};

pub fn setup() {
    let _ = TestLogger::init(LevelFilter::Debug, Config::default());
}

/// We run the synthesis separately to test that the lifetimes of the values
/// can be untied to the CircuitSynthesis struct. But also if we want to add LLZK tests
/// this makes sure to test the retargeability of the driver.
pub fn synthesize_and_generate_ir<'drv, F, C>(
    driver: &'drv mut Driver,
    circuit: C,
    params: IRGenParams<F>,
) -> ResolvedIRCircuit
where
    F: PrimeField,
    C: CircuitCallbacks<F>,
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

#[allow(dead_code)]
pub fn picus_test<F, C>(
    circuit: C,
    params: PicusParams,
    ir_params: IRGenParams<F>,
    expected: impl AsRef<str>,
    canonicalize: bool,
) where
    F: PrimeField,
    C: CircuitCallbacks<F>,
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
pub struct LookupCallbackHandler;

impl<F: Field> LookupCallbacks<F> for LookupCallbackHandler {
    fn on_lookup<'a>(
        &self,
        _lookup: Lookup<'a, F>,
        _table: &dyn LookupTableGenerator<F>,
        _temps: &mut Temps,
    ) -> anyhow::Result<IRStmt<ExprOrTemp<Cow<'a, Expression<F>>>>> {
        Ok(IRStmt::comment("Ignored lookup"))
    }
}
