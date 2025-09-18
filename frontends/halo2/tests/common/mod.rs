use std::borrow::Cow;

use ff::{Field, PrimeField};
use halo2_llzk_frontend::{
    driver::Driver,
    ir::{stmt::IRStmt, ResolvedIRCircuit},
    lookups::{
        callbacks::{LookupCallbacks, LookupTableGenerator},
        Lookup,
    },
    CircuitCallbacks, GateCallbacks, PicusParams,
};
use log::LevelFilter;
use midnight_halo2_proofs::plonk::Expression;
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
    lookups: Option<&dyn LookupCallbacks<F>>,
    gates: Option<&dyn GateCallbacks<F>>,
) -> ResolvedIRCircuit
where
    F: PrimeField,
    C: CircuitCallbacks<F>,
{
    let syn = driver.synthesize(&circuit).unwrap();
    let unresolved = driver.generate_ir(&syn, lookups, gates).unwrap();
    unresolved.resolve().unwrap()
}

pub fn picus_test<F, C>(
    circuit: C,
    params: PicusParams,
    lookups: Option<&dyn LookupCallbacks<F>>,
    gates: Option<&dyn GateCallbacks<F>>,
    expected: impl AsRef<str>,
    canonicalize: bool,
) where
    F: PrimeField,
    C: CircuitCallbacks<F>,
{
    let mut driver = Driver::default();
    let mut resolved = synthesize_and_generate_ir(&mut driver, circuit, lookups, gates);
    if canonicalize {
        resolved.constant_fold().unwrap();
        resolved.canonicalize();
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
    ) -> anyhow::Result<IRStmt<Cow<'a, Expression<F>>>> {
        Ok(IRStmt::comment("Ignored lookup"))
    }
}
