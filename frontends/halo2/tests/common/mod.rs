use std::{borrow::Cow, cell::RefCell};

use ff::{Field, PrimeField};
#[cfg(feature = "llzk-backend")]
use halo2_llzk_frontend::LlzkParams;
#[cfg(feature = "picus-backend")]
use halo2_llzk_frontend::PicusParams;
use halo2_llzk_frontend::{
    CircuitSynthesis, Synthesizer,
    driver::Driver,
    ir::{ResolvedIRCircuit, generate::IRGenParams, stmt::IRStmt},
    lookups::{Lookup, callbacks::LookupCallbacks, table::LookupTableGenerator},
    temps::{ExprOrTemp, Temps},
    to_plonk_error,
};
use halo2_proofs::plonk::{Any, Column};
use halo2_proofs::plonk::{Challenge, FloorPlanner};
use halo2_proofs::{
    circuit::{
        Value,
        groups::{GroupKey, RegionsGroup},
    },
    plonk::{
        Advice, Assignment, Circuit, ConstraintSystem, Error, Expression, Fixed, Instance, Selector,
    },
    utils::rational::Rational,
};
use llzk::prelude::*;
use log::LevelFilter;
use melior::ir::Module;
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
    params: IRGenParams<F, Expression<F>>,
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

fn common_lowering<F, C>(
    circuit: C,
    driver: &mut Driver,
    ir_params: IRGenParams<F, Expression<F>>,
    canonicalize: bool,
) -> ResolvedIRCircuit
where
    F: PrimeField,
    C: CircuitSynthesis<F, CS = ConstraintSystem<F>>,
{
    let mut resolved = synthesize_and_generate_ir(driver, circuit, ir_params);
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
    resolved
}

#[cfg(feature = "picus-backend")]
#[allow(dead_code)]
pub fn picus_test<F, C>(
    circuit: C,
    params: PicusParams,
    ir_params: IRGenParams<F, Expression<F>>,
    expected: impl AsRef<str>,
    canonicalize: bool,
) where
    F: PrimeField,
    C: CircuitSynthesis<F, CS = ConstraintSystem<F>>,
{
    let mut driver = Driver::default();
    let resolved = common_lowering(circuit, &mut driver, ir_params, canonicalize);
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

#[cfg(feature = "llzk-backend")]
pub fn check_llzk(
    driver: &Driver,
    circuit: &ResolvedIRCircuit,
    params: LlzkParams,
    expected_llzk: impl AsRef<str>,
) {
    let context = params.context();
    let output = driver.llzk(&circuit, params).unwrap();
    assert!(output.module().as_operation().verify());
    let output_str = format!("{}", output.module().as_operation());
    let expected = Module::parse(context, expected_llzk.as_ref())
        .expect("Failed to parse expected test output!");
    let expected_str = format!("{}", expected.as_operation());
    similar_asserts::assert_eq!(expected_str, output_str);
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

#[cfg(feature = "llzk-backend")]
#[allow(dead_code)]
pub fn llzk_test<F, C>(
    circuit: C,
    params: LlzkParams,
    ir_params: IRGenParams<F, Expression<F>>,
    expected_llzk: impl AsRef<str>,
    canonicalize: bool,
) where
    F: PrimeField,
    C: CircuitSynthesis<F, CS = ConstraintSystem<F>>,
{
    let mut driver = Driver::default();
    let resolved = common_lowering(circuit, &mut driver, ir_params, canonicalize);
    log::info!("Completed IR lowering!");
    log::logger().flush();
    check_llzk(&driver, &resolved, params, expected_llzk);
    log::info!("Completed transforming IR to LLZK!");
    log::logger().flush();
}

#[allow(dead_code)]
pub struct LookupCallbackHandler;

impl<F: Field> LookupCallbacks<F, Expression<F>> for LookupCallbackHandler {
    fn on_lookup<'a>(
        &self,
        _lookup: &'a Lookup<Expression<F>>,
        _table: &dyn LookupTableGenerator<F>,
        _temps: &mut Temps,
    ) -> anyhow::Result<IRStmt<ExprOrTemp<Cow<'a, Expression<F>>>>> {
        Ok(IRStmt::comment("Ignored lookup"))
    }
}

/// Implementation of Assignment for testing.
pub struct SynthesizerAssignment<'a, F: Field> {
    synthetizer: &'a mut Synthesizer<F>,
}

impl<'a, F: Field> SynthesizerAssignment<'a, F> {
    pub fn synthesize<C: Circuit<F>>(
        circuit: &C,
        config: C::Config,
        synthetizer: &'a mut Synthesizer<F>,
        cs: &ConstraintSystem<F>,
    ) -> Result<(), Error> {
        let mut assign = Self { synthetizer };
        let constants = cs.constants().clone();
        C::FloorPlanner::synthesize(&mut assign, circuit, config, constants)?;

        Ok(())
    }
}

impl<F: Field> Assignment<F> for SynthesizerAssignment<'_, F> {
    fn enter_region<NR, N>(&mut self, region_name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.synthetizer.enter_region(region_name().into());
    }

    fn exit_region(&mut self) {
        self.synthetizer.exit_region();
    }

    fn enable_selector<A, AR>(&mut self, _: A, selector: &Selector, row: usize) -> Result<(), Error>
    where
        AR: Into<String>,
        A: FnOnce() -> AR,
    {
        self.synthetizer.enable_selector(*selector, row);
        Ok(())
    }

    fn query_instance(&self, _column: Column<Instance>, _row: usize) -> Result<Value<F>, Error> {
        Ok(Value::unknown())
    }

    fn assign_advice<V, VR, A, AR>(
        &mut self,
        _name: A,
        advice: Column<Advice>,
        row: usize,
        _value: V,
    ) -> Result<(), Error>
    where
        VR: Into<Rational<F>>,
        AR: Into<String>,
        V: FnOnce() -> Value<VR>,
        A: FnOnce() -> AR,
    {
        self.synthetizer.on_advice_assigned(advice, row);
        Ok(())
    }

    fn assign_fixed<V, VR, A, AR>(
        &mut self,
        _: A,
        fixed: Column<Fixed>,
        row: usize,
        value: V,
    ) -> Result<(), Error>
    where
        VR: Into<Rational<F>>,
        AR: Into<String>,
        V: FnOnce() -> Value<VR>,
        A: FnOnce() -> AR,
    {
        let value = value().map(|f| f.into().evaluate());
        self.synthetizer.on_fixed_assigned(
            fixed,
            row,
            steal(&value).ok_or_else(|| {
                to_plonk_error(anyhow::anyhow!(
                    "Unknown value in fixed cell ({}, {row})",
                    fixed.index()
                ))
            })?,
        );
        Ok(())
    }

    fn copy(
        &mut self,
        from: Column<Any>,
        from_row: usize,
        to: Column<Any>,
        to_row: usize,
    ) -> Result<(), Error> {
        self.synthetizer.copy(from, from_row, to, to_row);
        Ok(())
    }

    fn fill_from_row(
        &mut self,
        column: Column<Fixed>,
        row: usize,
        value: Value<Rational<F>>,
    ) -> Result<(), Error> {
        self.synthetizer.fill_table(
            column,
            row,
            steal(&value.map(|f| f.evaluate())).ok_or_else(|| {
                to_plonk_error(anyhow::anyhow!(
                    "Unknown value in fixed cell ({}, {row})",
                    column.index()
                ))
            })?,
        );
        Ok(())
    }

    fn push_namespace<NR, N>(&mut self, name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.synthetizer.push_namespace(name().into());
    }

    fn pop_namespace(&mut self, name: Option<String>) {
        self.synthetizer.pop_namespace(name);
    }

    fn annotate_column<A, AR>(&mut self, _: A, _: Column<Any>)
    where
        AR: Into<String>,
        A: FnOnce() -> AR,
    {
    }

    fn get_challenge(&self, _: Challenge) -> Value<F> {
        Value::unknown()
    }

    fn enter_group<NR, N, K>(&mut self, name: N, key: K)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
        K: GroupKey,
    {
        self.synthetizer.enter_group(name().into(), key);
    }

    fn exit_group(&mut self, meta: RegionsGroup) {
        self.synthetizer.exit_group(meta)
    }
}

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
