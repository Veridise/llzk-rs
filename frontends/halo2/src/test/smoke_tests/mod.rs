#[cfg(feature = "midnight")]
pub mod midnight;

#[macro_export]
macro_rules! codegen_test {
    ($c:ty, $b:ty, $s:expr) => {{
        let circuit = <$c>::default();
        let backend = <$b>::initialize(Default::default());
        let output = backend.codegen_with_strat(&circuit, &$s);

        output.unwrap()
    }};
}

#[macro_export]
macro_rules! mock_codegen_test {
    ($c:ty) => {
        codegen_test!($c, MockBackend, CallGatesStrat)
    };
}

#[macro_export]
macro_rules! picus_codegen_test {
    ($c:ty) => {
        codegen_test!($c, PicusBackend<Lift<Fr>>, InlineConstraintsStrat)
    };
}
