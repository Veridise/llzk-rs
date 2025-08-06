use log::LevelFilter;
use simplelog::{Config, TestLogger};

use crate::backend::codegen::strats::inline::InlineConstraintsStrat;
use crate::backend::func::{ArgNo, FieldId};
use crate::backend::picus::{PicusBackend, PicusParamsBuilder};
use crate::backend::Backend;
use crate::halo2::{Field, Fr};
use crate::test::fixtures::midnight::fibonacci::FibonacciCircuit;
use crate::test::fixtures::midnight::lookup::LookupCircuit;
use crate::test::fixtures::midnight::lookup_2x3::Lookup2x3Circuit;
use crate::test::fixtures::midnight::lookup_2x3_fixed::Lookup2x3Circuit as Lookup2x3FixedCircuit;
use crate::test::fixtures::midnight::lookup_2x3_zerosel::Lookup2x3ZeroSelCircuit;
use crate::test::fixtures::midnight::mul::MulCircuit;
use crate::test::fixtures::midnight::mul_with_fixed_constraint::MulWithFixedConstraintCircuit;
use crate::test::mock::backend::{MockBackend, MockFunc, MockOutput};
use crate::test::mock::IRBuilder;
use crate::{codegen_test, mock_codegen_test, picus_codegen_test, Lift};

fn args(count: usize) -> Vec<ArgNo> {
    (0..count).map(Into::into).collect()
}

fn fields(count: usize) -> Vec<FieldId> {
    (0..count).map(Into::into).collect()
}

#[test]
fn test_mul_circuit_codegen() {
    let _ = TestLogger::init(LevelFilter::Debug, Config::default());
    let output = mock_codegen_test!(MulCircuit<Fr>);

    similar_asserts::assert_eq!(
        output,
        MockOutput {
            gates: vec![],
            main: Some(MockFunc {
                name: "Main".to_owned(),
                args: args(1),
                fields: fields(1),
                exprs: IRBuilder::default()
                    .push_const(Fr::ONE) //t0 := 1;
                    .push_const(-Fr::ONE) //t1 := -1;
                    .push_temp(0, 0) //t2 := temp(0, 0);
                    .product() //t3 := t1 * t2;
                    .push_temp(1, 0) //t4 := temp(1, 0);
                    .neg() //t5 := -t4;
                    .sum() //t6 := t3 + t5;
                    .product_with(Some(0), None) //t7 := t0 * t6;
                    .push_const(Fr::ZERO) //t8 := 0;
                    .push_const(Fr::ONE) //t9 := 1;
                    .push_temp(0, 0) //t10 := temp(0, 0);
                    .push_temp(1, 0) //t11 := temp(1, 0);
                    .product() //t12 := t10 * t11;
                    .push_temp(2, 0) //t13 := temp(2, 0);
                    .neg() //t14 := -t13;
                    .sum() //t15 := t12 + t14;
                    .product() //t16 := t9 * t15;
                    .push_const(Fr::ZERO) //t17 := 0;
                    .push_temp(0, 0) //t18 := temp(0, 0);
                    .push_arg(0) //t19 := arg0;
                    .push_temp(2, 0) //t20 := temp(2, 0);
                    .push_field(0) //t21 := field0;
                    .constraint(7, 8) //t22 := t7 == t8;
                    .constraint(16, 17) //t23 := t16 == t17;
                    .constraint(18, 19) //t24 := t18 == t19;
                    .constraint(20, 21) //t25 := t20 == t21;
                    .into()
            })
        }
    )
}

#[test]
fn test_mul_with_fixed_constraint_circuit_codegen() {
    let _ = TestLogger::init(LevelFilter::Debug, Config::default());
    let output = mock_codegen_test!(MulWithFixedConstraintCircuit<Fr>);

    similar_asserts::assert_eq!(
        output,
        MockOutput {
            gates: vec![],
            main: Some(MockFunc {
                name: "Main".to_owned(),
                args: args(1),
                fields: fields(1),
                exprs: IRBuilder::default()
                    .push_const(Fr::ONE) //t0 := 1;
                    .push_const(-Fr::ONE) //t1 := -1;
                    .push_temp(0, 0) //t2 := temp(0, 0);
                    .product() //t3 := t1 * t2;
                    .push_temp(1, 0) //t4 := temp(1, 0);
                    .neg() //t5 := -t4;
                    .sum() //t6 := t3 + t5;
                    .product_with(Some(0), None) //t7 := t0 * t6;
                    .push_const(Fr::ZERO) //t8 := 0;
                    .push_const(Fr::ONE) //t9 := 1;
                    .push_temp(0, 0) //t10 := temp(0, 0);
                    .push_temp(1, 0) //t11 := temp(1, 0);
                    .product() //t12 := t10 * t11;
                    .push_temp(2, 0) //t13 := temp(2, 0);
                    .neg() //t14 := -t13;
                    .sum() //t15 := t12 + t14;
                    .product() //t16 := t9 * t15;
                    .push_const(Fr::ZERO) //t17 := 0;
                    .push_const(Fr::ONE) //t18 := 1;
                    .push_const(-Fr::ONE) //t19 := -1;
                    .push_const(Fr::ONE + Fr::ONE) //t20 := 2;
                    .neg() //t21 := -t20;
                    .sum() //t22 := t19 + t21;
                    .product() //t23 := t18 * t22;
                    .push_const(Fr::ZERO) //t24 := 0
                    .push_temp(0, 0) //t25 := temp(0, 0);
                    .push_arg(0) //t26 := arg0;
                    .push_temp(2, 0) //t27 := temp(2, 0);
                    .push_field(0) //t28 := field0;
                    .push_fixed(0, 1) //t29 := fixed(0, 1);
                    .push_temp(3, 0) //t30 := temp(3, 0);
                    .push_fixed(0, 1) // t31 := fixed(0, 1);
                    .push_const(Fr::ONE + Fr::ONE) // t32 := 2;
                    .constraint(7, 8) //t33 := t7 == t8;
                    .constraint(16, 17) //t34 := t16 == t17;
                    .constraint(23, 24) //t35 := t23 == t24;
                    .constraint(25, 26) //t36 := t25 == t26;
                    .constraint(27, 28) //t37 := t27 == t28;
                    .constraint(29, 30) //t38 := t29 == t30;
                    .constraint(31, 32) //t39 := t31 == t32;
                    .into()
            })
        }
    )
}

macro_rules! picus_test {
    ($name:ident, $circ:ty) => {
        #[test]
        fn $name() {
            let _ = TestLogger::init(LevelFilter::Debug, Config::default());
            let output = picus_codegen_test!(
                $circ,
                PicusParamsBuilder::new()
                    .no_lift_fixed()
                    .short_names()
                    .no_optimize()
                    .into()
            );
            println!("{}", output.display());
        }
    };
}

picus_test!(test_mul_circuit_picus_codegen, MulCircuit<Lift<Fr>>);
picus_test!(test_lookup_circuit_picus_codegen, LookupCircuit<Lift<Fr>>);
picus_test!(
    test_lookup_circuit_picus_codegen_inlined_lookups,
    LookupCircuit<Lift<Fr>>
);

picus_test!(
    test_lookup_2x3_circuit_picus_codegen_inlined_lookups,
    Lookup2x3Circuit<Lift<Fr>>
);

picus_test!(
    test_lookup_2x3_circuit_picus_codegen,
    Lookup2x3Circuit<Lift<Fr>>
);

picus_test!(
    test_lookup_2x3_fixed_circuit_picus_codegen,
    Lookup2x3FixedCircuit<Lift<Fr>>
);

picus_test!(
    test_lookup_2x3_zerosel_circuit_picus_codegen_inlined_lookups,
    Lookup2x3ZeroSelCircuit<Lift<Fr>>
);

picus_test!(
    test_mul_with_fixed_constraint_circuit_picus_codegen,
    MulWithFixedConstraintCircuit<Lift<Fr>>
);
picus_test!(
    test_fibonacci_circuit_picus_codegen,
    FibonacciCircuit<Lift<Fr>>
);
