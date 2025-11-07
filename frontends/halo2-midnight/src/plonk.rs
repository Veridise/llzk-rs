//! Bridge types for types in the [`midnight_proofs::plonk`] module.

use crate::{
    macros::*,
    plonk::helper_traits::{ColumnConversion, ColumnWrapper, RotationExt as _, WrappedColumn},
};
use ff::Field;
use halo2_frontend_core::{
    expressions::{EvalExpression, EvaluableExpr, ExprBuilder, ExpressionInfo, ExpressionTypes},
    info_traits::{
        ChallengeInfo, ConstraintSystemInfo, CreateQuery, GateInfo, QueryInfo, SelectorInfo,
    },
    lookups::LookupData,
    table::Rotation,
};
use midnight_proofs::plonk::{
    Advice, AdviceQuery, Any, Challenge, Column, ColumnType, Expression, FirstPhase, Fixed,
    FixedQuery, Gate, Instance, InstanceQuery, Selector,
};

mod helper_traits {

    use midnight_proofs::{plonk::ColumnType, poly::Rotation};

    pub trait ColumnWrapper: Sized {
        type Src: ColumnType + Into<Self>;
    }

    pub trait WrappedColumn<W: ColumnWrapper<Src = Self>>: Sized {}

    impl<T, W> WrappedColumn<W> for T
    where
        W: ColumnWrapper<Src = T>,
        T: ColumnType + Into<W>,
    {
    }

    pub trait ColumnConversion<Dst: halo2_frontend_core::table::ColumnType>: Sized {
        fn convert(&self) -> Dst;
    }

    pub trait RotationExt {
        fn to_halo2(self) -> Rotation;
    }

    impl RotationExt for halo2_frontend_core::table::Rotation {
        fn to_halo2(self) -> Rotation {
            Rotation(self)
        }
    }
}

//===----------------------------------------------------------------------===//
// Advice
//===----------------------------------------------------------------------===//

newtype!(Advice, _Advice with Copy, Clone, Debug);

impl ColumnWrapper for _Advice {
    type Src = Advice;
}

impl ColumnConversion<halo2_frontend_core::query::Advice> for _Advice {
    fn convert(&self) -> halo2_frontend_core::query::Advice {
        halo2_frontend_core::query::Advice
    }
}

//===----------------------------------------------------------------------===//
// AdviceQuery
//===----------------------------------------------------------------------===//

newtype!(AdviceQuery, _AdviceQuery with Copy, Clone, Debug);

impl QueryInfo for _AdviceQuery {
    type Kind = halo2_frontend_core::query::Advice;

    fn rotation(&self) -> Rotation {
        self.0.rotation().0
    }

    fn column_index(&self) -> usize {
        self.0.column_index()
    }
}

impl<F: Field> CreateQuery<_Expression<F>> for _AdviceQuery {
    fn query_expr(index: usize, at: Rotation) -> _Expression<F> {
        Advice::query_cell(&Advice::new(FirstPhase), index, at.to_halo2()).into()
    }
}

//===----------------------------------------------------------------------===//
// Any
//===----------------------------------------------------------------------===//

newtype!(Any, _Any with Copy, Clone, Debug);

impl ColumnWrapper for _Any {
    type Src = Any;
}

impl ColumnConversion<halo2_frontend_core::table::Any> for _Any {
    fn convert(&self) -> halo2_frontend_core::table::Any {
        match self.0 {
            Any::Advice(_) => halo2_frontend_core::table::Any::Advice,
            Any::Fixed => halo2_frontend_core::table::Any::Fixed,
            Any::Instance => halo2_frontend_core::table::Any::Instance,
        }
    }
}

//===----------------------------------------------------------------------===//
// Challenge
//===----------------------------------------------------------------------===//

newtype!(Challenge, _Challenge with Copy, Clone, Debug);

impl ChallengeInfo for _Challenge {
    fn index(&self) -> usize {
        self.0.index()
    }

    fn phase(&self) -> u8 {
        self.0.phase()
    }
}

//===----------------------------------------------------------------------===//
// Column
//===----------------------------------------------------------------------===//

/// Newtype wrapper over [`Column`].
#[derive(Debug, Copy, Clone)]
pub struct _Column<C: ColumnWrapper>(Column<C::Src>);

impl<C: ColumnWrapper> From<Column<C::Src>> for _Column<C> {
    fn from(value: Column<C::Src>) -> Self {
        Self(value)
    }
}

impl<FC: ColumnWrapper + ColumnConversion<TC>, TC: halo2_frontend_core::table::ColumnType>
    From<_Column<FC>> for halo2_frontend_core::table::Column<TC>
{
    fn from(value: _Column<FC>) -> Self {
        let inner_column: FC::Src = *value.0.column_type();
        let column: FC = inner_column.into();
        Self::new(value.0.index(), column.convert())
    }
}

//===----------------------------------------------------------------------===//
// ConstraintSystem
//===----------------------------------------------------------------------===//

///Newtype wrapper over [`ConstraintSystem`](midnight_proofs::plonk::ConstraintSystem).
#[derive(Default, Debug)]
pub struct ConstraintSystem<F: Field> {
    cs: midnight_proofs::plonk::ConstraintSystem<F>,
    gates: Option<Vec<_Gate<F>>>,
    lookups: Option<Vec<(String, Vec<_Expression<F>>, Vec<_Expression<F>>)>>,
}

impl<F: Field> ConstraintSystem<F> {
    /// Returns a reference to the wrapped [`ConstraintSystem`](midnight_proofs::plonk::ConstraintSystem).
    pub fn inner(&self) -> &midnight_proofs::plonk::ConstraintSystem<F> {
        &self.cs
    }

    /// Returns a mutable reference to the wrapped [`ConstraintSystem`](midnight_proofs::plonk::ConstraintSystem).
    pub fn inner_mut(&mut self) -> &mut midnight_proofs::plonk::ConstraintSystem<F> {
        &mut self.cs
    }
}

impl<F: Field> ConstraintSystemInfo<F> for ConstraintSystem<F> {
    type Polynomial = _Expression<F>;

    fn synthesis_completed(&mut self) {
        let _ = self
            .gates
            .insert(self.cs.gates().iter().map(_Gate::new).collect());
        let _ = self.lookups.insert(
            self.cs
                .lookups()
                .iter()
                .map(|a| {
                    (
                        a.name().to_owned(),
                        a.input_expressions()
                            .iter()
                            .cloned()
                            .map(Into::into)
                            .collect(),
                        a.table_expressions()
                            .iter()
                            .cloned()
                            .map(Into::into)
                            .collect(),
                    )
                })
                .collect(),
        );
    }

    fn gates(&self) -> Vec<&dyn GateInfo<_Expression<F>>> {
        self.gates
            .iter()
            .flatten()
            .map(|g| g as &dyn GateInfo<_Expression<F>>)
            .collect()
    }

    fn lookups<'cs>(&'cs self) -> Vec<LookupData<'cs, _Expression<F>>> {
        self.lookups
            .iter()
            .flatten()
            .map(|(name, inputs, table)| LookupData {
                name: name.as_str(),
                arguments: inputs.as_slice(),
                table: table.as_slice(),
            })
            .collect()
    }
}

//===----------------------------------------------------------------------===//
// Expression
//===----------------------------------------------------------------------===//

/// Newtype wrapper over [`Expression`].
#[derive(Clone, Debug)]
pub struct _Expression<F: Field> {
    inner: Expression<F>,
    as_negation: Option<Box<_Expression<F>>>,
    as_fixed_query: Option<_FixedQuery>,
}

impl<F: Field> _Expression<F> {
    fn boxed(self) -> Box<Expression<F>> {
        Box::new(self.inner)
    }
}

impl<F: Field> From<Expression<F>> for _Expression<F> {
    fn from(inner: Expression<F>) -> Self {
        Self {
            as_negation: match &inner {
                Expression::Negated(expr) => Some(Box::new((**expr).clone().into())),
                _ => None,
            },
            as_fixed_query: match &inner {
                Expression::Fixed(query) => Some((*query).into()),
                _ => None,
            },
            inner,
        }
    }
}

impl<F: Field> ExpressionTypes for _Expression<F> {
    type Selector = _Selector;
    type FixedQuery = _FixedQuery;
    type AdviceQuery = _AdviceQuery;
    type InstanceQuery = _InstanceQuery;
    type Challenge = _Challenge;
}

impl<F: Field> ExpressionInfo for _Expression<F> {
    fn as_negation(&self) -> Option<&Self> {
        self.as_negation.as_deref()
    }

    fn as_fixed_query(&self) -> Option<&Self::FixedQuery> {
        self.as_fixed_query.as_ref()
    }
}

impl<F: Field> EvaluableExpr<F> for _Expression<F> {
    fn evaluate<E: EvalExpression<F, Self>>(&self, evaluator: &E) -> E::Output {
        self.inner.evaluate(
            &|f| evaluator.constant(&f),
            &|s| evaluator.selector(&s.into()),
            &|fq| evaluator.fixed(&_FixedQuery(fq)),
            &|aq| evaluator.advice(&_AdviceQuery(aq)),
            &|iq| evaluator.instance(&_InstanceQuery(iq)),
            &|c| evaluator.challenge(&_Challenge(c)),
            &|e| evaluator.negated(e),
            &|lhs, rhs| evaluator.sum(lhs, rhs),
            &|lhs, rhs| evaluator.product(lhs, rhs),
            &|lhs, rhs| evaluator.scaled(lhs, &rhs),
        )
    }
}

impl<F: Field> ExprBuilder<F> for _Expression<F> {
    fn constant(f: F) -> Self {
        Self::from(Expression::Constant(f))
    }

    fn selector(selector: <Self as ExpressionTypes>::Selector) -> Self {
        Self::from(Expression::Selector(selector.0))
    }

    fn fixed(fixed_query: <Self as ExpressionTypes>::FixedQuery) -> Self {
        Self::from(Expression::Fixed(fixed_query.0))
    }

    fn advice(advice_query: <Self as ExpressionTypes>::AdviceQuery) -> Self {
        Self::from(Expression::Advice(advice_query.0))
    }

    fn instance(instance_query: <Self as ExpressionTypes>::InstanceQuery) -> Self {
        Self::from(Expression::Instance(instance_query.0))
    }

    fn challenge(challenge: <Self as ExpressionTypes>::Challenge) -> Self {
        Self::from(Expression::Challenge(challenge.0))
    }

    fn negated(expr: Self) -> Self {
        Self::from(Expression::Negated(expr.boxed()))
    }

    fn sum(lhs: Self, rhs: Self) -> Self {
        Self::from(Expression::Sum(lhs.boxed(), rhs.boxed()))
    }

    fn product(lhs: Self, rhs: Self) -> Self {
        Self::from(Expression::Product(lhs.boxed(), rhs.boxed()))
    }

    fn scaled(lhs: Self, rhs: F) -> Self {
        Self::from(Expression::Scaled(lhs.boxed(), rhs))
    }
}

//===----------------------------------------------------------------------===//
// Fixed
//===----------------------------------------------------------------------===//

newtype!(Fixed, _Fixed with Copy, Clone, Debug);

impl ColumnWrapper for _Fixed {
    type Src = Fixed;
}

impl ColumnConversion<halo2_frontend_core::query::Fixed> for _Fixed {
    fn convert(&self) -> halo2_frontend_core::query::Fixed {
        halo2_frontend_core::query::Fixed
    }
}

//===----------------------------------------------------------------------===//
// FixedQuery
//===----------------------------------------------------------------------===//

newtype!(FixedQuery, _FixedQuery with Copy, Clone, Debug);

impl QueryInfo for _FixedQuery {
    type Kind = halo2_frontend_core::query::Fixed;

    fn rotation(&self) -> Rotation {
        self.0.rotation().0
    }

    fn column_index(&self) -> usize {
        self.0.column_index()
    }
}

impl<F: Field> CreateQuery<_Expression<F>> for _FixedQuery {
    fn query_expr(index: usize, at: Rotation) -> _Expression<F> {
        Fixed.query_cell(index, at.to_halo2()).into()
    }
}

//===----------------------------------------------------------------------===//
// Gate
//===----------------------------------------------------------------------===//

/// Newtype wrapper over [`Gate`].
#[derive(Debug)]
pub struct _Gate<F: Field> {
    name: String,
    polynomials: Vec<_Expression<F>>,
}

impl<F: Field> _Gate<F> {
    fn new(gate: &Gate<F>) -> Self {
        Self {
            name: gate.name().to_owned(),
            polynomials: gate.polynomials().iter().cloned().map(Into::into).collect(),
        }
    }
}

impl<F: Field> GateInfo<_Expression<F>> for _Gate<F> {
    fn name(&self) -> &str {
        &self.name
    }

    fn polynomials(&self) -> &[_Expression<F>] {
        &self.polynomials
    }
}

//===----------------------------------------------------------------------===//
// Instance
//===----------------------------------------------------------------------===//

newtype!(Instance, _Instance with Copy, Clone, Debug);

impl ColumnWrapper for _Instance {
    type Src = Instance;
}

impl ColumnConversion<halo2_frontend_core::query::Instance> for _Instance {
    fn convert(&self) -> halo2_frontend_core::query::Instance {
        halo2_frontend_core::query::Instance
    }
}

//===----------------------------------------------------------------------===//
// InstanceQuery
//===----------------------------------------------------------------------===//

newtype!(InstanceQuery, _InstanceQuery with Copy, Clone, Debug);

impl QueryInfo for _InstanceQuery {
    type Kind = halo2_frontend_core::query::Instance;

    fn rotation(&self) -> Rotation {
        self.0.rotation().0
    }

    fn column_index(&self) -> usize {
        self.0.column_index()
    }
}

impl<F: Field> CreateQuery<_Expression<F>> for _InstanceQuery {
    fn query_expr(index: usize, at: Rotation) -> _Expression<F> {
        Instance.query_cell(index, at.to_halo2()).into()
    }
}

//===----------------------------------------------------------------------===//
// Selector
//===----------------------------------------------------------------------===//

newtype!(Selector, _Selector with Copy, Clone, Debug);

impl SelectorInfo for _Selector {
    fn id(&self) -> usize {
        self.0.index()
    }
}
