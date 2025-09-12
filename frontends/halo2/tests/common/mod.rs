use std::borrow::Cow;

use ff::{Field, PrimeField};
use halo2_llzk_frontend::{
    driver::Driver,
    ir::stmt::IRStmt,
    lookups::{
        callbacks::{LookupCallbacks, LookupTableGenerator},
        Lookup,
    },
    CircuitCallbacks, PicusParams,
};
use log::LevelFilter;
use midnight_halo2_proofs::plonk::{Circuit, Expression};
use simplelog::{Config, TestLogger};

pub fn setup() {
    let _ = TestLogger::init(LevelFilter::Debug, Config::default());
}

pub fn picus_test<F, C>(circuit: C, params: PicusParams, expected: impl AsRef<str>)
where
    F: PrimeField,
    C: Circuit<F> + CircuitCallbacks<F, C>,
{
    let mut driver = Driver::default();
    driver.set_callbacks::<C>();
    let syn = driver.synthesize(&circuit).unwrap();

    let output = clean_string(
        &driver
            .picus(syn, params, None)
            .unwrap()
            .display()
            .to_string(),
    );
    let expected = clean_string(expected.as_ref());
    similar_asserts::assert_eq!(output, expected);
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
