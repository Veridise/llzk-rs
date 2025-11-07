use common::{picus::basic_picus_test, synthesis_impl};
use halo2_test_circuits::fibonacci;
use halo2curves::bn256::Fr;

mod common;

basic_picus_test! {
    fibonacci_circuit,
    FibonacciCircuitSynthesis::default(),
    include_str!("expected/picus/fibonacci.picus"),
    include_str!("expected/picus/fibonacci_opt.picus")
}

basic_picus_test! {
    fibonacci_grouped_circuit,
    GroupedFibonacciCircuitSynthesis::default(),
    include_str!("expected/picus/fibonacci_grouped.picus"),
    include_str!("expected/picus/fibonacci_grouped_opt.picus")
}

synthesis_impl!(
    FibonacciCircuitSynthesis,
    fibonacci::FibonacciCircuit<Fr>,
    [0, 1],
    [2]
);

synthesis_impl!(
    GroupedFibonacciCircuitSynthesis,
    fibonacci::grouped::FibonacciCircuit<Fr>,
    [0, 1],
    [2, 3]
);
