//! Traits for defining equivalence relations between types

/// Defines an equivalence relation between two entities.
pub trait EqvRelation<L, R = L> {
    fn equivalent(lhs: &L, rhs: &R) -> bool;
}

impl<L, R, E: EqvRelation<L, R>> EqvRelation<Vec<L>, Vec<R>> for E {
    fn equivalent(lhs: &Vec<L>, rhs: &Vec<R>) -> bool {
        std::iter::zip(lhs.iter(), rhs.iter()).all(|(lhs, rhs)| E::equivalent(lhs, rhs))
    }
}

impl<L, R, E: EqvRelation<L, R>> EqvRelation<Box<L>, Box<R>> for E {
    fn equivalent(lhs: &Box<L>, rhs: &Box<R>) -> bool {
        E::equivalent(lhs.as_ref(), rhs.as_ref())
    }
}

/// Equivalence relation on symbolic equivalence.
///
/// Symbolic in this context means that when comparing
/// entities information that does not affect the semantics
/// of what the entities are expression is ignored.
pub struct SymbolicEqv;
