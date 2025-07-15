#[cfg(feature = "midnight")]
pub mod midnight;

#[macro_export]
macro_rules! codegen_test {
    ($c:ty, $b:ty, $s:ty) => {{
        let circuit = <$c>::default();
        let backend = <$b>::initialize(Default::default());
        let output = backend.codegen_with_strat::<_, $s>(&circuit);

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
        codegen_test!($c, PicusBackend<Lift<Fr>>, InlineConstraintsStrat)
    };
}
