#[cfg(feature = "llzk-backend")]
mod inner {
    use ff::PrimeField;
    use halo2_llzk_frontend::{
        CircuitSynthesis, LlzkParams, LlzkParamsBuilder,
        driver::Driver,
        ir::{ResolvedIRCircuit, generate::IRGenParams},
    };
    use halo2_midnight_integration::plonk::{_Expression, ConstraintSystem};
    use llzk::prelude::{LlzkContext, OperationLike as _};

    use crate::common::common_lowering;

    #[allow(dead_code)]
    pub fn llzk_params(ctx: &LlzkContext) -> LlzkParams<'_> {
        LlzkParamsBuilder::new(ctx).no_optimize().build()
    }

    #[allow(dead_code)]
    pub fn opt_llzk_params(ctx: &LlzkContext) -> LlzkParams<'_> {
        LlzkParamsBuilder::new(ctx).build()
    }

    #[allow(dead_code)]
    pub fn check_llzk(
        driver: &Driver,
        circuit: &ResolvedIRCircuit,
        params: LlzkParams,
        expected_llzk: impl AsRef<str>,
    ) {
        let output = driver.llzk(&circuit, params).unwrap();
        assert!(output.module().as_operation().verify());
        mlir_testutils::assert_module_eq(output.module(), expected_llzk.as_ref());
    }

    #[allow(dead_code)]
    pub fn llzk_test<F, C>(
        circuit: C,
        params: LlzkParams,
        ir_params: IRGenParams<F, _Expression<F>>,
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
}

#[cfg(feature = "llzk-backend")]
#[allow(unused)]
pub use inner::*;

#[allow(unused_macros)]
macro_rules! basic_llzk_test {
    ($name:ident, $circuit:expr, $expected:expr, $expected_opt:expr, $ir_params:expr $(,)?) => {
        paste::paste! {
        #[cfg(feature = "llzk-backend")]
        mod [< $name _llzk >] {
            use super::*;
            #[test]
            fn no_opt() {
                common::setup();
                let ctx = llzk::context::LlzkContext::new();
                common::llzk::llzk_test(
                    $circuit,
                    common::llzk::llzk_params(&ctx),
                    $ir_params,
                    $expected,
                    false,
                );
            }

            #[test]
            fn opt() {
                common::setup();
                let ctx = llzk::context::LlzkContext::new();
                common::llzk::llzk_test(
                    $circuit,
                    common::llzk::opt_llzk_params(&ctx),
                    $ir_params,
                    $expected_opt,
                    true,
                );
            }
        }
        }
    };
    ($name:ident, $circuit:expr, $expected:expr, $expected_opt:expr $(,)?) => {
        $crate::common::llzk::basic_llzk_test! {
            $name,
            $circuit,
            $expected,
            $expected_opt,
            halo2_llzk_frontend::ir::generate::IRGenParamsBuilder::new().build()
        }
    };
}

#[allow(unused)]
pub(crate) use basic_llzk_test;
