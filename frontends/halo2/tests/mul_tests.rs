#[cfg(feature = "picus-backend")]
use common::picus::basic_picus_test;
use common::synthesis_impl;
use halo2_llzk_frontend::ir::generate::IRGenParamsBuilder;
use halo2_test_circuits::mul;
use halo2curves::bn256::Fr;

mod common;

basic_picus_test! {
    mul_circuit,
    MulCircuitSynthesis::default(),
    include_str!("expected/picus/mul_circuit.picus"),
    include_str!("expected/picus/mul_circuit_opt.picus")
}

basic_picus_test! {
    mul_flipped,
    MulFlippedCircuitSynthesis::default(),
    include_str!("expected/picus/mul_flipped_constraint.picus"),
    include_str!("expected/picus/mul_flipped_constraint_opt.picus")
}

basic_picus_test! {
    mul_fixed,
    MulFixedConstraintCircuitSynthesis::default(),
    include_str!("expected/picus/mul_with_fixed_constraint.picus"),
    include_str!("expected/picus/mul_with_fixed_constraint_opt.picus")
}

basic_picus_test! {
    recursive_groups,
    RecursiveMulCircuitSynthesis::default(),
    include_str!("expected/picus/recursive_groups.picus"),
    include_str!("expected/picus/recursive_groups_opt.picus")
}

// This test makes sure that the order in which input and output variables are printed is
// the same as their declaration order.
basic_picus_test! {
    ten_plus_io,
    TenPlusIOCircuitSynthesis::default(),
    include_str!("expected/picus/ten_plus_io.picus"),
    include_str!("expected/picus/ten_plus_io_opt.picus")
}

basic_picus_test! {
    grouped,
    GroupedMulsCircuitSynthesis::default(),
    include_str!("expected/picus/grouped_muls.picus"),
    include_str!("expected/picus/grouped_muls_opt.picus")
}

basic_picus_test! {
    different_bodies,
    DifferentBodiesCircuitSynthesis::default(),
    include_str!("expected/picus/different_bodies.picus"),
    include_str!("expected/picus/different_bodies_opt.picus")
}

basic_picus_test! {
    same_body,
    SameBodyCircuitSynthesis::default(),
    include_str!("expected/picus/same_body.picus"),
    include_str!("expected/picus/same_body_opt.picus")
}

basic_picus_test! {
    deep_callstack,
    DeepCallstackCircuitSynthesis::default(),
    include_str!("expected/picus/deep_callstack.picus"),
    include_str!("expected/picus/deep_callstack_opt.picus")
}

basic_picus_test! {
    mul_rewriter,
    MulCircuitSynthesis::default(),
    include_str!("expected/picus/mul_with_rewriter.picus"),
    include_str!("expected/picus/mul_with_rewriter_opt.picus"),
    IRGenParamsBuilder::new().gate_callbacks(&common::GC).build()
}

#[cfg(feature = "picus-backend")]
mod mul_inject {
    use halo2_frontend_core::table::RegionIndex;
    use halo2_llzk_frontend::CircuitSynthesis;
    use halo2_llzk_frontend::{
        driver::Driver,
        expressions::ExpressionInRow,
        ir::{CmpOp, ResolvedIRCircuit, stmt::IRStmt},
    };
    use halo2_midnight_integration::plonk::_Expression;
    use halo2_midnight_integration::plonk::ConstraintSystem;
    use halo2_proofs::plonk::Expression;

    use super::*;

    const EXPECTED_PICUS: &'static str = include_str!("expected/picus/mul_inject.picus");
    const EXPECTED_OPT_PICUS: &'static str = include_str!("expected/picus/mul_inject_opt.picus");

    fn ir_to_inject<'e>() -> Vec<(RegionIndex, IRStmt<ExpressionInRow<'e, _Expression<Fr>>>)> {
        let mut cs = ConstraintSystem::<Fr>::default();
        let config = MulInjectCircuitSynthesis::configure(&mut cs);
        let a = config.col_a.cur();
        let hundrend = Expression::Constant(Fr::from(1000));
        let stmts = [
            IRStmt::constraint(CmpOp::Lt, a.clone(), hundrend.clone())
                .map(&|e| ExpressionInRow::new(0, _Expression::from(e))),
            IRStmt::constraint(CmpOp::Ge, a, hundrend)
                .map(&|e| ExpressionInRow::new(1, _Expression::from(e))),
        ];

        let mut injected = vec![];
        for row in 0..6 {
            let index = RegionIndex::from(row / 2);
            let offset = row % 2;

            let payload = (index, stmts[offset].clone());
            log::debug!("payload = {payload:?}");
            injected.push(payload);
        }
        injected
    }

    macro_rules! ensure_validation {
        ($x:expr) => {{
            let (status, errs) = $x.validate();

            if status.is_err() {
                for err in errs {
                    log::error!("{err}");
                }
                panic!("Test failed due to validation errors");
            }
        }};
    }

    fn generate_ir(driver: &mut Driver) -> ResolvedIRCircuit {
        let circuit = MulInjectCircuitSynthesis::default();
        let syn = driver.synthesize(&circuit).unwrap();

        let mut unresolved = driver
            .generate_ir(&syn, IRGenParamsBuilder::new().build())
            .unwrap();
        let ir = ir_to_inject();
        unresolved.inject_ir(ir, &syn).unwrap();
        ensure_validation!(unresolved);
        let resolved = unresolved.resolve().unwrap();
        ensure_validation!(resolved);
        resolved
    }

    #[test]
    fn picus() {
        common::setup();
        let mut driver = Driver::default();
        let resolved = generate_ir(&mut driver);

        common::picus::check_picus(
            &driver,
            &resolved,
            common::picus::picus_params(),
            EXPECTED_PICUS,
        );
    }

    #[test]
    fn opt_picus() {
        common::setup();
        let mut driver = Driver::default();
        let mut resolved = generate_ir(&mut driver);

        resolved.constant_fold().unwrap();
        ensure_validation!(resolved);
        resolved.canonicalize();
        ensure_validation!(resolved);

        common::picus::check_picus(
            &driver,
            &resolved,
            common::picus::opt_picus_params(),
            EXPECTED_OPT_PICUS,
        );
    }
}

synthesis_impl!(MulCircuitSynthesis, mul::MulCircuit<Fr>, [0], [1]);
synthesis_impl!(
    DeepCallstackCircuitSynthesis,
    mul::grouped::deep_callstack::MulCircuit<Fr>,
    [0],
    [1]
);
synthesis_impl!(
    SameBodyCircuitSynthesis,
    mul::grouped::same_body::MulCircuit<Fr>,
    [0],
    [1]
);
synthesis_impl!(
    DifferentBodiesCircuitSynthesis,
    mul::grouped::different_bodies::MulCircuit<Fr>,
    [0],
    [1]
);
synthesis_impl!(
    GroupedMulsCircuitSynthesis,
    mul::grouped::MulCircuit<Fr>,
    [0],
    [1]
);
synthesis_impl!(
    TenPlusIOCircuitSynthesis,
    mul::ten_plus_io::MulCircuit<Fr>,
    Vec::from_iter(0..=10),
    Vec::from_iter(11..=21)
);
synthesis_impl!(
    RecursiveMulCircuitSynthesis,
    mul::recursive_groups::MulCircuit<Fr>,
    [0, 1, 2, 3],
    [4]
);
synthesis_impl!(
    MulFixedConstraintCircuitSynthesis,
    mul::fixed_constraint::MulWithFixedConstraintCircuit<Fr>,
    [0],
    [1]
);
synthesis_impl!(
    MulInjectCircuitSynthesis,
    mul::injection::MulCircuit<Fr>,
    [0],
    [1, 2, 3]
);
synthesis_impl!(
    MulFlippedCircuitSynthesis,
    mul::flipped_constraint::MulCircuit<Fr>,
    [0],
    [1]
);
