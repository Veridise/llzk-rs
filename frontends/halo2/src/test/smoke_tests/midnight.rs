use crate::backend::func::{ArgNo, FieldId};
use crate::backend::Backend;
use crate::halo2::{Field, Fr};
use crate::test::fixtures::midnight::fibonacci::FibonacciCircuit;
use crate::test::fixtures::midnight::mul::MulCircuit;
use crate::test::mock::backend::{MockBackend, MockExprIR, MockFunc, MockOutput};
use crate::{codegen_test, mock_codegen_test};

#[test]
fn test_mul_circuit_codegen() {
    let output = mock_codegen_test!(MulCircuit<Fr>);

    assert_eq!(
        output,
        MockOutput {
            gates: vec![MockFunc {
                name: "mul".to_owned(),
                args: vec![
                    ArgNo::from(0), // s
                    ArgNo::from(1), // a
                    ArgNo::from(2), // b
                    ArgNo::from(3), // c
                    ArgNo::from(4)  // f
                ],
                fields: vec![],
                exprs: vec![
                    MockExprIR::Arg(ArgNo::from(0)), // 0  = s
                    MockExprIR::Arg(ArgNo::from(4)), // 1  = f
                    MockExprIR::Arg(ArgNo::from(1)), // 2  = a
                    MockExprIR::Product(2, 1),       // 3  = f * a
                    MockExprIR::Arg(ArgNo::from(2)), // 4  = b
                    MockExprIR::Neg(4),              // 5  = -b
                    MockExprIR::Sum(3, 5),           // 6  = f * a + (-b)
                    MockExprIR::Product(6, 0),       // 7  = s * (f * a + (-b))
                    MockExprIR::Arg(ArgNo::from(0)), // 8  = s
                    MockExprIR::Arg(ArgNo::from(1)), // 9  = a
                    MockExprIR::Arg(ArgNo::from(2)), // 10 = b
                    MockExprIR::Product(10, 9),      // 11 = b * a
                    MockExprIR::Arg(ArgNo::from(3)), // 12 = c
                    MockExprIR::Neg(12),             // 13 = -c
                    MockExprIR::Sum(11, 13),         // 14 = b * a + (-c)
                    MockExprIR::Product(14, 8),      // 15 = s * (b * a + (-c))
                    MockExprIR::Const(Fr::ZERO),     // 16 = 0
                    MockExprIR::Constraint(7, 16),   // 17
                    MockExprIR::Constraint(15, 16)   // 18
                ]
            }],
            main: Some(MockFunc {
                name: "Main".to_owned(),
                args: vec![ArgNo::from(0)],
                fields: vec![FieldId::from(0)],
                exprs: vec![
                    MockExprIR::Const(Fr::ONE),                              // 0 = 1
                    MockExprIR::Temp(0, 0),                                  // 1 = t0
                    MockExprIR::Temp(1, 0),                                  // 2 = t1
                    MockExprIR::Temp(2, 0),                                  // 3 = t2
                    MockExprIR::Const(-Fr::ONE),                             // 4 = -1
                    MockExprIR::Temp(0, 0),                                  // 5 = t0
                    MockExprIR::Arg(ArgNo::from(0)),                         // 6 = a0
                    MockExprIR::Temp(2, 0),                                  // 7 = t2
                    MockExprIR::Field(FieldId::from(0)),                     // 8 = f0
                    MockExprIR::Call("mul".to_owned(), vec![0, 1, 2, 3, 4]), // 9
                    MockExprIR::Constraint(5, 6),                            // 10
                    MockExprIR::Constraint(7, 8)                             // 11
                ]
            })
        }
    )
}

#[test]
fn test_fibonacci_circuit_codegen() {
    mock_codegen_test!(FibonacciCircuit<Fr>);
}

//#[test]
//fn test_mul_circuit_llzk_codegen() {
//    smoke_test!(MulCircuit<Fr>, LLZKBackend);
//}
//
//#[test]
//fn test_fibonacci_circuit_llzk_codegen() {
//    smoke_test!(FibonacciCircuit<Fr>, LLZKBackend);
//}
