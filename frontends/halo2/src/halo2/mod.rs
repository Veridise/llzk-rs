//! Opaque module that exposes the correct halo2 library based on the implementation selected via

use ff::Field;

use crate::{
    expressions::{EvalExpression, EvaluableExpr, ExprBuilder, ExpressionInfo, ExpressionTypes},
    info_traits::{
        ChallengeInfo, ConstraintSystemInfo, CreateQuery, GateInfo, GroupInfo, QueryInfo,
        SelectorInfo,
    },
    lookups::LookupData,
    synthesis::regions::RegionIndex,
    table::{Cell, Rotation, RotationExt},
};

impl RotationExt<halo2_proofs::poly::Rotation> for Rotation {
    fn cur() -> halo2_proofs::poly::Rotation {
        halo2_proofs::poly::Rotation::cur()
    }

    #[cfg(test)]
    fn next() -> halo2_proofs::poly::Rotation {
        halo2_proofs::poly::Rotation::next()
    }
}

/// Temporary implementation of [`QueryInfo`] for [`halo2_proofs::plonk::FixedQuery`]
impl QueryInfo for halo2_proofs::plonk::FixedQuery {
    type Kind = crate::resolvers::Fixed;

    fn rotation(&self) -> Rotation {
        self.rotation().0
    }

    fn column_index(&self) -> usize {
        self.column_index()
    }
}

/// Temporary implementation of [`CreateQuery`] for [`halo2_proofs::plonk::FixedQuery`]
impl<F: Field> CreateQuery<halo2_proofs::plonk::Expression<F>> for halo2_proofs::plonk::FixedQuery {
    fn query_expr(index: usize, at: Rotation) -> halo2_proofs::plonk::Expression<F> {
        use halo2_proofs::plonk::ColumnType as _;
        halo2_proofs::plonk::Fixed::query_cell(
            &halo2_proofs::plonk::Fixed,
            index,
            halo2_proofs::poly::Rotation(at),
        )
    }
}

/// Temporary implementation of [`QueryInfo`] for [`halo2_proofs::plonk::AdviceQuery`]
impl QueryInfo for halo2_proofs::plonk::AdviceQuery {
    type Kind = crate::resolvers::Advice;

    fn rotation(&self) -> Rotation {
        self.rotation().0
    }

    fn column_index(&self) -> usize {
        self.column_index()
    }
}

/// Temporary implementation of [`CreateQuery`] for [`halo2_proofs::plonk::AdviceQuery`]
impl<F: Field> CreateQuery<halo2_proofs::plonk::Expression<F>>
    for halo2_proofs::plonk::AdviceQuery
{
    fn query_expr(index: usize, at: Rotation) -> halo2_proofs::plonk::Expression<F> {
        use halo2_proofs::plonk::ColumnType as _;
        halo2_proofs::plonk::Advice::query_cell(
            &halo2_proofs::plonk::Advice::new(halo2_proofs::plonk::FirstPhase),
            index,
            halo2_proofs::poly::Rotation(at),
        )
    }
}

/// Temporary implementation of [`QueryInfo`] for [`halo2_proofs::plonk::InstanceQuery`]
impl QueryInfo for halo2_proofs::plonk::InstanceQuery {
    type Kind = crate::resolvers::Instance;

    fn rotation(&self) -> Rotation {
        self.rotation().0
    }

    fn column_index(&self) -> usize {
        self.column_index()
    }
}

/// Temporary implementation of [`CreateQuery`] for [`halo2_proofs::plonk::InstanceQuery`]
impl<F: Field> CreateQuery<halo2_proofs::plonk::Expression<F>>
    for halo2_proofs::plonk::InstanceQuery
{
    fn query_expr(index: usize, at: Rotation) -> halo2_proofs::plonk::Expression<F> {
        use halo2_proofs::plonk::ColumnType as _;
        halo2_proofs::plonk::Instance::query_cell(
            &halo2_proofs::plonk::Instance,
            index,
            halo2_proofs::poly::Rotation(at),
        )
    }
}

/// Temporary implementation of [`SelectorInfo`] for [`halo2_proofs::plonk::Selector`]
impl SelectorInfo for halo2_proofs::plonk::Selector {
    fn id(&self) -> usize {
        self.index()
    }
}

/// Temporary implementation of [`ChallengeInfo`] for [`halo2_proofs::plonk::Challenge`]
impl ChallengeInfo for halo2_proofs::plonk::Challenge {
    fn index(&self) -> usize {
        self.index()
    }

    fn phase(&self) -> u8 {
        self.phase()
    }
}

/// Temporary implementation of [`GroupInfo`] for [`halo2_proofs::circuit::groups::RegionsGroup`].
impl GroupInfo for halo2_proofs::circuit::groups::RegionsGroup {
    fn inputs(&self) -> impl Iterator<Item = Cell> + '_ {
        self.inputs().map(|c| Cell {
            region_index: RegionIndex::from(*c.region_index),
            row_offset: c.row_offset,
            column: c.column.into(),
        })
    }

    fn outputs(&self) -> impl Iterator<Item = Cell> + '_ {
        self.outputs().map(|c| Cell {
            region_index: RegionIndex::from(*c.region_index),
            row_offset: c.row_offset,
            column: c.column.into(),
        })
    }
}

/// Temporary implementation of [`ConstraintSystemInfo`].
impl<F: Field> ConstraintSystemInfo<F> for halo2_proofs::plonk::ConstraintSystem<F> {
    type Polynomial = halo2_proofs::plonk::Expression<F>;

    fn gates(&self) -> Vec<&dyn GateInfo<halo2_proofs::plonk::Expression<F>>> {
        self.gates()
            .iter()
            .map(|g| g as &dyn GateInfo<halo2_proofs::plonk::Expression<F>>)
            .collect()
    }

    fn lookups<'cs>(&'cs self) -> Vec<LookupData<'cs, halo2_proofs::plonk::Expression<F>>> {
        self.lookups()
            .iter()
            .map(|a| LookupData {
                name: a.name(),
                arguments: a.input_expressions(),
                table: a.table_expressions(),
            })
            .collect()
    }
}

/// Temporary implementation of [`GateInfo`].
impl<F: Field> GateInfo<halo2_proofs::plonk::Expression<F>> for halo2_proofs::plonk::Gate<F> {
    fn name(&self) -> &str {
        self.name()
    }

    fn polynomials(&self) -> &[halo2_proofs::plonk::Expression<F>] {
        self.polynomials()
    }
}

/// Temporary implementation of [`ExpressionTypes`].
impl<F: Field> ExpressionTypes for halo2_proofs::plonk::Expression<F> {
    type Selector = halo2_proofs::plonk::Selector;
    type FixedQuery = halo2_proofs::plonk::FixedQuery;
    type AdviceQuery = halo2_proofs::plonk::AdviceQuery;
    type InstanceQuery = halo2_proofs::plonk::InstanceQuery;
    type Challenge = halo2_proofs::plonk::Challenge;
}

/// Temporary implementation of [`ExpressionInfo`].
impl<F: Field> ExpressionInfo for halo2_proofs::plonk::Expression<F> {
    fn as_negation(&self) -> Option<&Self> {
        match self {
            Self::Negated(expr) => Some(&**expr),
            _ => None,
        }
    }

    fn as_fixed_query(&self) -> Option<&Self::FixedQuery> {
        match self {
            Self::Fixed(query) => Some(query),
            _ => None,
        }
    }
}

/// Temporary implementation of [`EvaluableExpr`].
impl<F: Field> EvaluableExpr<F> for halo2_proofs::plonk::Expression<F> {
    fn evaluate<E: EvalExpression<F, Self>>(&self, evaluator: &E) -> E::Output {
        self.evaluate(
            &|f| evaluator.constant(&f),
            &|s| evaluator.selector(&s),
            &|fq| evaluator.fixed(&fq),
            &|aq| evaluator.advice(&aq),
            &|iq| evaluator.instance(&iq),
            &|c| evaluator.challenge(&c),
            &|e| evaluator.negated(e),
            &|lhs, rhs| evaluator.sum(lhs, rhs),
            &|lhs, rhs| evaluator.product(lhs, rhs),
            &|lhs, rhs| evaluator.scaled(lhs, &rhs),
        )
    }
}

/// Temporary implementation of [`ExprBuilder`].
impl<F: Field> ExprBuilder<F> for halo2_proofs::plonk::Expression<F> {
    fn constant(f: F) -> Self {
        Self::Constant(f)
    }

    fn selector(selector: <Self as ExpressionTypes>::Selector) -> Self {
        Self::Selector(selector)
    }

    fn fixed(fixed_query: <Self as ExpressionTypes>::FixedQuery) -> Self {
        Self::Fixed(fixed_query)
    }

    fn advice(advice_query: <Self as ExpressionTypes>::AdviceQuery) -> Self {
        Self::Advice(advice_query)
    }

    fn instance(instance_query: <Self as ExpressionTypes>::InstanceQuery) -> Self {
        Self::Instance(instance_query)
    }

    fn challenge(challenge: <Self as ExpressionTypes>::Challenge) -> Self {
        Self::Challenge(challenge)
    }

    fn negated(expr: Self) -> Self {
        Self::Negated(Box::new(expr))
    }

    fn sum(lhs: Self, rhs: Self) -> Self {
        Self::Sum(Box::new(lhs), Box::new(rhs))
    }

    fn product(lhs: Self, rhs: Self) -> Self {
        Self::Product(Box::new(lhs), Box::new(rhs))
    }

    fn scaled(lhs: Self, rhs: F) -> Self {
        Self::Scaled(Box::new(lhs), rhs)
    }
}
