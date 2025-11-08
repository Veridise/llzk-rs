use ff::{Field, PrimeField};
use halo2_llzk_frontend::{
    CircuitSynthesis,
    driver::Driver,
    gates::{GateCallbacks, GateRewritePattern, GateScope, RewriteError},
    ir::{ResolvedIRCircuit, generate::IRGenParams},
};
use halo2_midnight_integration::plonk::{_Expression, ConstraintSystem};

pub mod llzk;
pub mod picus;

pub fn setup() {
    let _ = simplelog::TestLogger::init(log::LevelFilter::Debug, simplelog::Config::default());
}

#[macro_export]
macro_rules! ensure_validation {
    ($x:expr) => {{
        let (status, errs) = $x.validate();

        if status.is_err() {
            for err in errs {
                log::error!("{err}");
            }
            panic!("Test failed due to validation errors");
        }
    }};
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
    ensure_validation!(unresolved);
    let resolved = unresolved.resolve().unwrap();
    ensure_validation!(resolved);
    resolved
}

fn common_lowering<F, C>(
    circuit: C,
    driver: &mut Driver,
    ir_params: IRGenParams<F, _Expression<F>>,
    canonicalize: bool,
) -> ResolvedIRCircuit
where
    F: PrimeField,
    C: CircuitSynthesis<F, CS = ConstraintSystem<F>>,
{
    let mut resolved = synthesize_and_generate_ir(driver, circuit, ir_params);
    if canonicalize {
        resolved.constant_fold().unwrap();
        ensure_validation!(resolved);
        resolved.canonicalize();
        ensure_validation!(resolved);
    }
    resolved
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

struct DummyPattern;

impl<F: Field> GateRewritePattern<F, _Expression<F>> for DummyPattern {
    fn match_gate<'a>(
        &self,
        _gate: GateScope<'a, '_, F, _Expression<F>>,
    ) -> Result<(), RewriteError>
    where
        F: Field,
    {
        Err(RewriteError::NoMatch)
    }
}

pub struct GC;

impl<F: Field> GateCallbacks<F, _Expression<F>> for GC {
    fn patterns(&self) -> Vec<Box<dyn GateRewritePattern<F, _Expression<F>>>>
    where
        F: Field,
    {
        vec![Box::new(DummyPattern)]
    }
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
                    cs,
                )
            }
        }
    };
}

pub(crate) use synthesis_impl;
