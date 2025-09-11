#[cfg(feature = "midnight")]
pub mod midnight;

//#[macro_export]
//macro_rules! codegen_test {
//    ($c:ty, $b:ident, $s:ty, $params:expr) => {};
//}
//
//#[macro_export]
//macro_rules! mock_codegen_test {
//    ($c:ty) => {
//        codegen_test!($c, MockBackend, InlineConstraintsStrat)
//    };
//}

#[macro_export]
macro_rules! picus_codegen_test {
    ($c:ty, $params:expr) => {{
        let circuit = <$c>::default();
        let mut driver = crate::driver::Driver::default();
        driver.set_callbacks::<$c>();
        let syn = driver.synthesize(&circuit).unwrap();

        let output = driver.test_picus(
            syn,
            $params,
            crate::backend::codegen::strats::groups::GroupConstraintsStrat {},
        );

        output.unwrap()
    }};
}

//#[macro_export]
//macro_rules! llzk_codegen_test {
//    ($c:ty, $params:expr) => {
//        codegen_test!($c, LlzkBackend<Fr>, InlineConstraintsStrat, $params)
//    };
//}
