use std::marker::PhantomData;

use super::mock::backend::{MockBackend, MockOutput};
use crate::backend::Backend;
use crate::halo2::Fr;
use crate::CircuitWithIO;

#[cfg(feature = "midnight")]
pub mod midnight;

//pub fn run_codegen_test<'a, C, B, P, O>(params: P) -> O
//where
//    C: CircuitWithIO<Fr> + Default,
//    B: Backend<'a, P, O>,
//    P: Default,
//{
//    let circuit = C::default();
//    let backend = B::initialize(params);
//    let output = backend.codegen_impl(&circuit);
//
//    assert!(output.is_ok());
//
//    output.unwrap()
//}

//pub fn run_mock_codegen_test<C>() -> MockOutput
//where
//    C: CircuitWithIO<Fr> + Default,
//{
//    let circuit = C::default();
//    let backend = MockBackend::initialize(());
//    let output = backend.codegen_impl(&circuit);
//
//    assert!(output.is_ok());
//
//    output.unwrap()
//    //run_codegen_test::<C, MockBackend, (), MockOutput>(())
//}

#[macro_export]
macro_rules! codegen_test {
    ($c:ty, $b:ty) => {{
        let circuit = <$c>::default();
        let backend = <$b>::initialize(());
        let output = backend.codegen_impl(&circuit);

        //assert!(output.is_ok());

        output.unwrap()
    }};
}

#[macro_export]
macro_rules! mock_codegen_test {
    ($c:ty) => {
        codegen_test!($c, MockBackend)
    };
}
