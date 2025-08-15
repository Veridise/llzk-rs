#[cfg(feature = "midnight")]
pub mod midnight;

#[macro_export]
macro_rules! codegen_test {
    ($c:ty, $b:ty, $s:ty, $params:expr) => {{
        let circuit = <$c>::default();
        let backend = <$b>::initialize($params);
        let output = backend.codegen_with_strat::<_, $c, $s>(&circuit);

        output.unwrap()
    }};
}

#[macro_export]
macro_rules! mock_codegen_test {
    ($c:ty) => {
        codegen_test!($c, MockBackend, InlineConstraintsStrat)
    };
}

#[macro_export]
macro_rules! picus_codegen_test {
    ($c:ty) => {
        codegen_test!($c, PicusBackend<Fr>, InlineConstraintsStrat)
    };
    ($c:ty, $params:expr) => {
        codegen_test!($c, PicusBackend<Fr>, InlineConstraintsStrat, $params)
    };
}

#[macro_export]
macro_rules! llzk_codegen_test {
    ($c:ty, $params:expr) => {
        codegen_test!($c, LlzkBackend<Fr>, InlineConstraintsStrat, $params)
    };
}
