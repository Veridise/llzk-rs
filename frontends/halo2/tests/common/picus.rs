#[cfg(feature = "picus-backend")]
mod inner {
    use crate::common::{clean_string, common_lowering};
    use ff::PrimeField;
    use halo2_llzk_frontend::{
        CircuitSynthesis, PicusParams, PicusParamsBuilder,
        driver::Driver,
        ir::{ResolvedIRCircuit, generate::IRGenParams},
    };
    use halo2_midnight_integration::plonk::{_Expression, ConstraintSystem};

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
        let resolved = common_lowering(circuit, &mut driver, ir_params, canonicalize);
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
}

#[cfg(feature = "picus-backend")]
pub use inner::*;

#[allow(unused_macros)]
macro_rules! basic_picus_test {
    ($name:ident, $circuit:expr, $expected:expr, $expected_opt:expr, $ir_params:expr $(,)?) => {
        paste::paste! {
        #[cfg(feature = "picus-backend")]
        mod [< $name _picus >] {
            use super::*;
            #[test]
            fn no_opt() {
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
            fn opt() {
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
