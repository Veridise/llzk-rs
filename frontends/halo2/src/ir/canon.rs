//! Functions related to canonicalization of the IR.

use crate::ir::{expr::IRAexpr, CmpOp};

pub fn canonicalize_constraint(
    op: CmpOp,
    lhs: &IRAexpr,
    rhs: &IRAexpr,
) -> Option<(CmpOp, IRAexpr, IRAexpr)> {
    match (op, lhs, rhs) {
        // (= (+ X (- Y)) 0) => (= X Y) OR (= (+ (- X) Y) 0) => (= X Y)
        (CmpOp::Eq, IRAexpr::Sum(sum_lhs, sum_rhs), IRAexpr::Constant(zero)) if *zero == 0usize => {
            if let IRAexpr::Negated(y) = &**sum_rhs {
                return Some((CmpOp::Eq, (**sum_lhs).clone(), (**y).clone()));
            }
            if let IRAexpr::Negated(y) = &**sum_lhs {
                return Some((CmpOp::Eq, (**y).clone(), (**sum_rhs).clone()));
            }
            None
        }
        // (= (* 1 (+ X (- Y))) 0) => (= X Y) OR (= (* 1 (+ (- X) Y)) 0) => (= X Y)
        (CmpOp::Eq, IRAexpr::Product(mul_lhs, mul_rhs), IRAexpr::Constant(zero))
            if *zero == 0usize =>
        {
            match (&**mul_lhs, &**mul_rhs) {
                (IRAexpr::Constant(one), IRAexpr::Sum(sum_lhs, sum_rhs)) if *one == 1usize => {
                    if let IRAexpr::Negated(y) = &**sum_rhs {
                        return Some((CmpOp::Eq, (**sum_lhs).clone(), (**y).clone()));
                    }
                    if let IRAexpr::Negated(y) = &**sum_lhs {
                        return Some((CmpOp::Eq, (**y).clone(), (**sum_rhs).clone()));
                    }
                    None
                }
                _ => None,
            }
        }
        // Nothing matched
        _ => None,
    }
}
