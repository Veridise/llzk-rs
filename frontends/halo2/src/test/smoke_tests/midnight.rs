use log::LevelFilter;
use simplelog::{Config, TestLogger};

use crate::backend::codegen::strats::inline::InlineConstraintsStrat;
use crate::backend::func::{ArgNo, FieldId};
use crate::backend::llzk::{LlzkBackend, LlzkParamsBuilder};
use crate::backend::picus::{PicusBackend, PicusParamsBuilder};
use crate::backend::Backend;
use crate::halo2::{Field, Fr};
use crate::test::fixtures::midnight::fibonacci::FibonacciCircuit;
use crate::test::fixtures::midnight::grouped_muls::MulCircuit as GroupedMulsCircuit;
use crate::test::fixtures::midnight::grouped_muls2::MulCircuit as GroupedMulsCircuit2;
use crate::test::fixtures::midnight::grouped_muls3::MulCircuit as GroupedMulsCircuit3;
use crate::test::fixtures::midnight::lookup::LookupCircuit;
use crate::test::fixtures::midnight::lookup_2x3::Lookup2x3Circuit;
use crate::test::fixtures::midnight::lookup_2x3_fixed::Lookup2x3Circuit as Lookup2x3FixedCircuit;
use crate::test::fixtures::midnight::lookup_2x3_zerosel::Lookup2x3ZeroSelCircuit;
use crate::test::fixtures::midnight::mul::MulCircuit;
use crate::test::fixtures::midnight::mul_with_fixed_constraint::MulWithFixedConstraintCircuit;
use crate::test::fixtures::midnight::mul_with_rewriter::MulCircuit as MulWithRewriterCircuit;
//use crate::test::mock::backend::{MockBackend, MockFunc, MockOutput};
//use crate::test::mock::IRBuilder;
use crate::picus_codegen_test;

macro_rules! picus_test {
    ($name:ident, $circ:ty) => {
        #[test]
        fn $name() {
            let _ = TestLogger::init(LevelFilter::Debug, Config::default());
            let output = picus_codegen_test!(
                $circ,
                PicusParamsBuilder::new()
                    .short_names()
                    .no_optimize()
                    .build()
            );
            println!("{}", output.display());
        }
    };
}

macro_rules! llzk_test {
    ($name:ident, $circ:ty) => {
        #[test]
        fn $name() {
            let _ = TestLogger::init(LevelFilter::Debug, Config::default());
            let ctx = llzk::LlzkContext::new();
            log::debug!("ctx = {ctx:?}");
            let output = llzk_codegen_test!($circ, LlzkParamsBuilder::new(&ctx).build());
            println!("{}", output);
        }
    };
}

picus_test!(test_mul_circuit_picus_codegen, MulCircuit<Fr>);
picus_test!(
    test_grouped_mul_circuit_picus_codegen,
    GroupedMulsCircuit<Fr>
);
picus_test!(
    test_grouped_mul2_circuit_picus_codegen,
    GroupedMulsCircuit2<Fr>
);
picus_test!(
    test_grouped_mul3_circuit_picus_codegen,
    GroupedMulsCircuit3<Fr>
);
//llzk_test!(test_mul_circuit_llzk_codegen, MulCircuit<Fr>);
picus_test!(
    test_mul_with_rewriter_circuit_picus_codegen,
    MulWithRewriterCircuit<Fr>
);
picus_test!(test_lookup_circuit_picus_codegen, LookupCircuit<Fr>);
picus_test!(
    test_lookup_circuit_picus_codegen_inlined_lookups,
    LookupCircuit<Fr>
);

picus_test!(
    test_lookup_2x3_circuit_picus_codegen_inlined_lookups,
    Lookup2x3Circuit<Fr>
);

picus_test!(test_lookup_2x3_circuit_picus_codegen, Lookup2x3Circuit<Fr>);

picus_test!(
    test_lookup_2x3_fixed_circuit_picus_codegen,
    Lookup2x3FixedCircuit<Fr>
);

picus_test!(
    test_lookup_2x3_zerosel_circuit_picus_codegen_inlined_lookups,
    Lookup2x3ZeroSelCircuit<Fr>
);

picus_test!(
    test_mul_with_fixed_constraint_circuit_picus_codegen,
    MulWithFixedConstraintCircuit<Fr>
);
picus_test!(test_fibonacci_circuit_picus_codegen, FibonacciCircuit<Fr>);
