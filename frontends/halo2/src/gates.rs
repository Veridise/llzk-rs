use std::cmp::Ordering;
use std::collections::HashSet;
use std::hash::Hash;

use crate::halo2::*;

fn find_in_binop<'a, QR, F, Q>(lhs: &'a Expression<F>, rhs: &'a Expression<F>, q: Q) -> HashSet<QR>
where
    F: Field,
    Q: Fn(&'a Expression<F>) -> HashSet<QR>,
    QR: Hash + Eq + Clone,
{
    q(lhs).union(&q(rhs)).cloned().collect()
}

fn find_selectors<F: Field>(poly: &Expression<F>) -> HashSet<&Selector> {
    match poly {
        Expression::Selector(selector) => [selector].into(),
        Expression::Negated(expression) => find_selectors(expression),
        Expression::Sum(lhs, rhs) => find_in_binop(lhs, rhs, find_selectors),
        Expression::Product(lhs, rhs) => find_in_binop(lhs, rhs, find_selectors),
        Expression::Scaled(expression, _) => find_selectors(expression),
        _ => Default::default(),
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AnyQuery {
    Advice(AdviceQuery),
    Instance(InstanceQuery),
    Fixed(FixedQuery),
}

impl AnyQuery {
    pub fn column_index(&self) -> usize {
        match self {
            AnyQuery::Advice(advice_query) => advice_query.column_index(),
            AnyQuery::Instance(instance_query) => instance_query.column_index(),
            AnyQuery::Fixed(fixed_query) => fixed_query.column_index(),
        }
    }

    pub fn rotation(&self) -> Rotation {
        match self {
            AnyQuery::Advice(advice_query) => advice_query.rotation(),
            AnyQuery::Instance(instance_query) => instance_query.rotation(),
            AnyQuery::Fixed(fixed_query) => fixed_query.rotation(),
        }
    }

    pub fn phase(&self) -> Option<u8> {
        match self {
            AnyQuery::Advice(advice_query) => Some(advice_query.phase()),
            _ => None,
        }
    }

    pub fn type_id(&self) -> u8 {
        match self {
            AnyQuery::Advice(_) => 0,
            AnyQuery::Instance(_) => 1,
            AnyQuery::Fixed(_) => 2,
        }
    }

    pub fn to_tuple(&self) -> (u8, usize, i32, Option<u8>) {
        (
            self.type_id(),
            self.column_index(),
            self.rotation().0,
            self.phase(),
        )
    }
}

impl PartialEq<FixedQuery> for AnyQuery {
    fn eq(&self, other: &FixedQuery) -> bool {
        match self {
            Self::Fixed(query) => query == other,
            _ => false,
        }
    }
}

impl PartialEq<InstanceQuery> for AnyQuery {
    fn eq(&self, other: &InstanceQuery) -> bool {
        match self {
            Self::Instance(query) => query == other,
            _ => false,
        }
    }
}

impl PartialEq<AdviceQuery> for AnyQuery {
    fn eq(&self, other: &AdviceQuery) -> bool {
        match self {
            Self::Advice(query) => query == other,
            _ => false,
        }
    }
}

impl PartialOrd for AnyQuery {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AnyQuery {
    /// Column metadata is ordered lexicographically by column type, then index, and lastly
    /// rotation. In the case of Advice columns the last element is the phase.
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_tuple().cmp(&other.to_tuple())
    }
}

impl Hash for AnyQuery {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let (typ, col, rot, pha) = self.to_tuple();
        typ.hash(state);
        col.hash(state);
        rot.hash(state);
        pha.hash(state);
    }
}

impl From<&AdviceQuery> for AnyQuery {
    fn from(query: &AdviceQuery) -> Self {
        Self::Advice(*query)
    }
}

impl From<&InstanceQuery> for AnyQuery {
    fn from(query: &InstanceQuery) -> Self {
        Self::Instance(*query)
    }
}

impl From<&FixedQuery> for AnyQuery {
    fn from(query: &FixedQuery) -> Self {
        Self::Fixed(*query)
    }
}

impl From<AdviceQuery> for AnyQuery {
    fn from(query: AdviceQuery) -> Self {
        Self::Advice(query)
    }
}

impl From<InstanceQuery> for AnyQuery {
    fn from(query: InstanceQuery) -> Self {
        Self::Instance(query)
    }
}

impl From<FixedQuery> for AnyQuery {
    fn from(query: FixedQuery) -> Self {
        Self::Fixed(query)
    }
}

fn find_queries<F: Field>(poly: &Expression<F>) -> HashSet<AnyQuery> {
    match poly {
        Expression::Advice(query) => [query.into()].into(),
        Expression::Instance(query) => [query.into()].into(),
        Expression::Fixed(query) => [query.into()].into(),
        Expression::Negated(expression) => find_queries(expression),
        Expression::Sum(lhs, rhs) => find_in_binop(lhs, rhs, find_queries),
        Expression::Product(lhs, rhs) => find_in_binop(lhs, rhs, find_queries),
        Expression::Scaled(expression, _) => find_queries(expression),
        _ => Default::default(),
    }
}

pub type GateArity<'a> = (Vec<&'a Selector>, Vec<AnyQuery>);

pub fn find_gate_selector_set<F: Field>(constraints: &[Expression<F>]) -> HashSet<&Selector> {
    constraints.iter().flat_map(find_selectors).collect()
}

pub fn find_gate_query_selector_set<F: Field>(constraints: &[Expression<F>]) -> HashSet<AnyQuery> {
    constraints.iter().flat_map(find_queries).collect()
}

pub fn find_gate_selectors<F: Field>(constraints: &[Expression<F>]) -> Vec<&Selector> {
    let mut selectors: Vec<&Selector> = find_gate_selector_set(constraints)
        .iter()
        .copied()
        .collect();
    selectors.sort_by_key(|lhs| lhs.index());
    selectors
}

pub fn compute_gate_arity<'a, F: Field>(constraints: &'a [Expression<F>]) -> GateArity<'a> {
    let mut queries: Vec<AnyQuery> = find_gate_query_selector_set(constraints)
        .into_iter()
        .collect();
    queries.sort();
    (find_gate_selectors(constraints), queries)
}
