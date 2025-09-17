//! Functions related to canonicalization of the IR.

use crate::ir::{expr::IRAexpr, CmpOp};

pub fn canonicalize_constraint(
    op: CmpOp,
    lhs: &IRAexpr,
    rhs: &IRAexpr,
) -> Option<(CmpOp, IRAexpr, IRAexpr)> {
    match (op, lhs, rhs) {
        // (= (+ X (- Y)) 0) => (= X Y)
        (CmpOp::Eq, IRAexpr::Sum(x, sum_rhs), IRAexpr::Constant(zero)) if *zero == 0usize => {
            match &**sum_rhs {
                IRAexpr::Negated(y) => Some((CmpOp::Eq, (**x).clone(), (**y).clone())),
                _ => None,
            }
        }
        // (= (* 1 (+ X (- Y))) 0) => (= X Y)
        (CmpOp::Eq, IRAexpr::Product(mul_lhs, mul_rhs), IRAexpr::Constant(zero))
            if *zero == 0usize =>
        {
            match (&**mul_lhs, &**mul_rhs) {
                (IRAexpr::Constant(one), IRAexpr::Sum(x, sum_rhs)) if *one == 1usize => {
                    match &**sum_rhs {
                        IRAexpr::Negated(y) => Some((CmpOp::Eq, (**x).clone(), (**y).clone())),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        // Nothing matched
        _ => None,
    }
}
