use std::marker::PhantomData;

use ff::Field;

use crate::{
    expressions::{EvalExpression, EvaluableExpr, ExprBuilder, ExpressionInfo, ExpressionTypes},
    info_traits::{QueryInfo, SelectorInfo},
    resolvers::{Fixed, ResolvedQuery, ResolvedSelector, ResolversProvider},
};

pub struct ConstantFolding<'a, F: Field, E> {
    resolvers: &'a dyn ResolversProvider<F>,
    _marker: PhantomData<E>,
}

impl<'a, F: Field, E> ConstantFolding<'a, F, E> {
    pub fn new(resolvers: &'a dyn ResolversProvider<F>) -> Self {
        Self {
            resolvers,
            _marker: Default::default(),
        }
    }
}

impl<F: Field, E> ConstantFolding<'_, F, E>
where
    E: ExprBuilder<F> + ExpressionInfo + Clone + ExpressionTypes,
{
    fn resolve_selector(&self, selector: &dyn SelectorInfo) -> Option<F> {
        match self
            .resolvers
            .selector_resolver()
            .resolve_selector(selector)
            .ok()?
        {
            ResolvedSelector::Const(bool) => Some(bool.to_f::<F>()),
            ResolvedSelector::Arg(_) => None,
        }
    }

    fn resolve_fixed_query(&self, fixed_query: &dyn QueryInfo<Kind = Fixed>) -> Option<F> {
        match self
            .resolvers
            .query_resolver()
            .resolve_fixed_query(fixed_query)
            .ok()?
        {
            ResolvedQuery::Lit(f) => Some(f),
            ResolvedQuery::IO(_) => None,
        }
    }

    pub fn constant_fold(&self, e: &E) -> E
    where
        E: EvaluableExpr<F>,
    {
        match e.evaluate(self) {
            Ok(f) => E::constant(f),
            Err(e) => e,
        }
    }
}

impl<F: Field, E> EvalExpression<F, E> for ConstantFolding<'_, F, E>
where
    E: ExprBuilder<F> + ExpressionInfo + Clone + ExpressionTypes,
{
    type Output = Result<F, E>;

    fn constant(&self, f: &F) -> Self::Output {
        Ok(*f)
    }

    fn selector(&self, selector: &E::Selector) -> Self::Output {
        self.resolve_selector(selector)
            .inspect(|f| log::debug!("Folded selector {selector:?} to constant {f:?}"))
            .ok_or_else(|| E::selector(*selector))
    }

    fn fixed(&self, fixed_query: &E::FixedQuery) -> Self::Output {
        self.resolve_fixed_query(fixed_query)
            .inspect(|f| log::debug!("Folded fixed query {fixed_query:?} to constant {f:?}"))
            .ok_or_else(|| E::fixed(*fixed_query))
    }

    fn advice(&self, advice_query: &E::AdviceQuery) -> Self::Output {
        Err(E::advice(*advice_query))
    }

    fn instance(&self, instance_query: &E::InstanceQuery) -> Self::Output {
        Err(E::instance(*instance_query))
    }

    fn challenge(&self, challenge: &E::Challenge) -> Self::Output {
        Err(E::challenge(*challenge))
    }

    fn negated(&self, expr: Self::Output) -> Self::Output {
        expr.map(|f| -f)
            .map_err(|e| e.as_negation().cloned().unwrap_or_else(|| E::negated(e)))
    }

    fn sum(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output {
        match (lhs, rhs) {
            (Ok(lhs), Ok(rhs)) => Ok(lhs + rhs),
            (Ok(lhs), Err(rhs)) if lhs == F::ZERO => Err(rhs),
            (Ok(lhs), Err(rhs)) => Err(E::sum(E::constant(lhs), rhs)),
            (Err(lhs), Ok(rhs)) if rhs == F::ZERO => Err(lhs),
            (Err(lhs), Ok(rhs)) => Err(E::sum(lhs, E::constant(rhs))),
            (Err(lhs), Err(rhs)) => Err(E::sum(lhs, rhs)),
        }
    }

    fn product(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output {
        match (lhs, rhs) {
            (Ok(lhs), Ok(rhs)) => Ok(lhs * rhs),
            (Ok(lhs), Err(_)) if lhs == F::ZERO => Ok(F::ZERO),
            (Ok(lhs), Err(rhs)) if lhs == F::ONE => Err(rhs),
            (Ok(lhs), Err(rhs)) if lhs == -F::ONE => self.negated(Err(rhs)),
            (Ok(lhs), Err(rhs)) => Err(E::product(E::constant(lhs), rhs)),
            (Err(_), Ok(rhs)) if rhs == F::ZERO => Ok(F::ZERO),
            (Err(lhs), Ok(rhs)) if rhs == F::ONE => Err(lhs),
            (Err(lhs), Ok(rhs)) if rhs == -F::ONE => self.negated(Err(lhs)),
            (Err(lhs), Ok(rhs)) => Err(E::product(lhs, E::constant(rhs))),
            (Err(lhs), Err(rhs)) => Err(E::product(lhs, rhs)),
        }
    }

    fn scaled(&self, lhs: Self::Output, rhs: &F) -> Self::Output {
        let rhs = *rhs;
        if rhs == F::ZERO {
            return Ok(F::ZERO);
        }
        if rhs == F::ONE {
            return lhs;
        }
        if rhs == -F::ONE {
            return self.negated(lhs);
        }
        match lhs {
            Ok(lhs) => Ok(lhs * rhs),
            Err(lhs) => Err(E::scaled(lhs, rhs)),
        }
    }
}
