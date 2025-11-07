use ff::PrimeField;
use halo2_llzk_frontend::{
    CircuitSynthesis, PicusParams, PicusParamsBuilder,
    driver::Driver,
    ir::{ResolvedIRCircuit, generate::IRGenParams},
};

pub fn picus_params() -> PicusParams {
    PicusParamsBuilder::new()
        .short_names()
        .no_optimize()
        .build()
}

pub fn opt_picus_params() -> PicusParams {
    PicusParamsBuilder::new().short_names().build()
}

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

#[allow(unused_macros)]
macro_rules! basic_picus_test {
    ($name:ident, $circuit:expr, $expected:expr, $expected_opt:expr, $ir_params:expr $(,)?) => {
        #[cfg(feature = "picus-backend")]
        mod $name {
            use super::*;
            #[test]
            fn picus() {
                common::setup();
                common::picus::picus_test(
                    $circuit,
                    common::picus::picus_params(),
                    $ir_params,
                    $expected,
                    false,
                );
            }

            #[test]
            fn opt_picus() {
                common::setup();
                common::picus::picus_test(
                    $circuit,
                    common::picus::opt_picus_params(),
                    $ir_params,
                    $expected_opt,
                    true,
                );
            }
        }
    };
    ($name:ident, $circuit:expr, $expected:expr, $expected_opt:expr $(,)?) => {
        $crate::common::picus::basic_picus_test! {
            $name,
            $circuit,
            $expected,
            $expected_opt,
            halo2_llzk_frontend::ir::generate::IRGenParamsBuilder::new().build()
        }
    };
}

pub(crate) use basic_picus_test;
use halo2_midnight_integration::plonk::{_Expression, ConstraintSystem};

use crate::common::{clean_string, synthesize_and_generate_ir};
