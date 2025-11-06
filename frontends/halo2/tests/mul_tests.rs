use common::synthesis_impl;
use group::ff::Field;
#[cfg(feature = "picus-backend")]
use halo2_llzk_frontend::PicusParamsBuilder;
use halo2_llzk_frontend::ir::generate::IRGenParamsBuilder;
use halo2_llzk_frontend::{AdviceIO, CircuitIO, CircuitSynthesis, InstanceIO};
use halo2_midnight_integration::Wrapped;
use halo2_midnight_integration::plonk::{_Column, _Instance, ConstraintSystem};
use halo2_midnight_integration::synthesizer::SynthesizerAssignment;
use halo2_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value};
use halo2_proofs::plonk::{Advice, Circuit, Column, Error, Fixed, Instance, Selector};
use halo2_proofs::poly::Rotation;
use halo2curves::bn256::Fr;
use std::marker::PhantomData;

use halo2_test_circuits::mul;

mod common;

common::basic_picus_test! {
    mul_circuit,
    MulCircuitSynthesis::default(),
    include_str!("expected/picus/mul_circuit.picus"),
    include_str!("expected/picus/mul_circuit_opt.picus")
}

common::basic_picus_test! {
    mul_flipped,
    MulFlippedCircuitSynthesis::default(),
    include_str!("expected/picus/mul_flipped_constraint.picus"),
    include_str!("expected/picus/mul_flipped_constraint_opt.picus")
}

common::basic_picus_test! {
    mul_fixed,
    MulFixedConstraintCircuitSynthesis::default(),
    include_str!("expected/picus/mul_with_fixed_constraint.picus"),
    include_str!("expected/picus/mul_with_fixed_constraint_opt.picus")
}

common::basic_picus_test! {
    recursive_groups,
    RecursiveMulCircuitSynthesis::default(),
    include_str!("expected/picus/recursive_groups.picus"),
    include_str!("expected/picus/recursive_groups_opt.picus")
}

// This test makes sure that the order in which input and output variables are printed is
// the same as their declaration order.
common::basic_picus_test! {
    ten_plus_io,
    TenPlusIOCircuitSynthesis::default(),
    include_str!("expected/picus/ten_plus_io.picus"),
    include_str!("expected/picus/ten_plus_io_opt.picus")
}

common::basic_picus_test! {
    grouped,
    GroupedMulsCircuitSynthesis::default(),
    include_str!("expected/picus/grouped_muls.picus"),
    include_str!("expected/picus/grouped_muls_opt.picus")
}

common::basic_picus_test! {
    different_bodies,
    DifferentBodiesCircuitSynthesis::default(),
    include_str!("expected/picus/different_bodies.picus"),
    include_str!("expected/picus/different_bodies_opt.picus")
}

common::basic_picus_test! {
    same_body,
    SameBodyCircuitSynthesis::default(),
    include_str!("expected/picus/same_body.picus"),
    include_str!("expected/picus/same_body_opt.picus")
}

common::basic_picus_test! {
    deep_callstack,
    DeepCallstackCircuitSynthesis::default(),
    include_str!("expected/picus/deep_callstack.picus"),
    include_str!("expected/picus/deep_callstack_opt.picus")
}

mod mul_rewriter {
    use halo2_llzk_frontend::gates::{GateCallbacks, GateRewritePattern, GateScope, RewriteError};
    use halo2_midnight_integration::plonk::_Expression;

    use super::*;
    struct DummyPattern;

    impl<F: Field> GateRewritePattern<F, _Expression<F>> for DummyPattern {
        fn match_gate<'a>(
            &self,
            _gate: GateScope<'a, '_, F, _Expression<F>>,
        ) -> Result<(), RewriteError>
        where
            F: Field,
        {
            Err(RewriteError::NoMatch)
        }
    }

    struct GC;

    impl<F: Field> GateCallbacks<F, _Expression<F>> for GC {
        fn patterns(&self) -> Vec<Box<dyn GateRewritePattern<F, _Expression<F>>>>
        where
            F: Field,
        {
            vec![Box::new(DummyPattern)]
        }
    }

    #[cfg(feature = "picus-backend")]
    #[test]
    fn picus() {
        common::setup();
        common::picus_test(
            MulCircuitSynthesis::default(),
            common::picus_params(),
            IRGenParamsBuilder::new().gate_callbacks(&GC).build(),
            include_str!("expected/picus/mul_with_rewriter.picus"),
            false,
        );
    }
    #[cfg(feature = "picus-backend")]
    #[test]
    fn opt_picus() {
        common::setup();
        common::picus_test(
            MulCircuitSynthesis::default(),
            common::opt_picus_params(),
            IRGenParamsBuilder::new().gate_callbacks(&GC).build(),
            include_str!("expected/picus/mul_with_rewriter_opt.picus"),
            true,
        );
    }
}

mod mul_inject {
    use halo2_llzk_frontend::{
        RegionIndex,
        expressions::ExpressionInRow,
        ir::{CmpOp, stmt::IRStmt},
    };
    use halo2_midnight_integration::plonk::_Expression;
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

    use halo2_llzk_frontend::driver::Driver;
    #[cfg(feature = "picus-backend")]
    #[test]
    fn picus() {
        common::setup();
        let circuit = MulInjectCircuitSynthesis::default();
        let mut driver = Driver::default();
        let resolved = {
            let syn = driver.synthesize(&circuit).unwrap();

            let mut unresolved = driver
                .generate_ir(&syn, IRGenParamsBuilder::new().build())
                .unwrap();
            let ir = ir_to_inject();
            unresolved.inject_ir(ir, &syn).unwrap();
            let (status, errs) = unresolved.validate();
            if status.is_err() {
                for err in errs {
                    log::error!("{err}");
                }
                panic!("Test failed due to validation errors");
            }
            let resolved = unresolved.resolve().unwrap();
            let (status, errs) = resolved.validate();
            if status.is_err() {
                for err in errs {
                    log::error!("{err}");
                }
                panic!("Test failed due to validation errors");
            }
            resolved
        };

        common::check_picus(
            &driver,
            &resolved,
            PicusParamsBuilder::new()
                .short_names()
                .no_optimize()
                .build(),
            EXPECTED_PICUS,
        );
    }

    #[cfg(feature = "picus-backend")]
    #[test]
    fn opt_picus() {
        common::setup();
        {
            let circuit = MulInjectCircuitSynthesis::default();
            let mut driver = Driver::default();
            let mut resolved = {
                let syn = driver.synthesize(&circuit).unwrap();

                let mut unresolved = driver
                    .generate_ir(&syn, IRGenParamsBuilder::new().build())
                    .unwrap();
                let ir = ir_to_inject();
                unresolved.inject_ir(ir, &syn).unwrap();
                let (status, errs) = unresolved.validate();
                if status.is_err() {
                    for err in errs {
                        log::error!("{err}");
                    }
                    panic!("Test failed due to validation errors");
                }
                let resolved = unresolved.resolve().unwrap();
                let (status, errs) = resolved.validate();
                if status.is_err() {
                    for err in errs {
                        log::error!("{err}");
                    }
                    panic!("Test failed due to validation errors");
                }
                resolved
            };
            resolved.constant_fold().unwrap();
            let (status, errs) = resolved.validate();
            if status.is_err() {
                for err in errs {
                    log::error!("{err}");
                }
                panic!("Test failed due to validation errors");
            }
            resolved.canonicalize();
            let (status, errs) = resolved.validate();
            if status.is_err() {
                for err in errs {
                    log::error!("{err}");
                }
                panic!("Test failed due to validation errors");
            }
            common::check_picus(
                &driver,
                &resolved,
                PicusParamsBuilder::new().short_names().build(),
                EXPECTED_OPT_PICUS,
            );
        };
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
