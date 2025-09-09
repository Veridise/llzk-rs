use midnight_halo2_proofs::plonk::{
    Expression, FixedQuery, Selector,
};

use crate::{
    backend::resolvers::{ResolvedQuery, ResolvedSelector, ResolversProvider},
    expressions::utils::ExprDebug,
    halo2::Field,
};

use super::rewriter::ExpressionRewriter;

pub struct ConstantFolding<'a, F: Field> {
    resolvers: &'a dyn ResolversProvider<F>,
}

impl<'a, F: Field> ConstantFolding<'a, F> {
    pub fn new(resolvers: &'a dyn ResolversProvider<F>) -> Self {
        Self { resolvers }
    }
}

fn expr_as_const<F: Copy>(e: &Expression<F>) -> Option<F> {
    match e {
        Expression::Constant(f) => Some(*f),
        _ => None,
    }
}

fn sum_patterns<F: Field>(lhs: &Expression<F>, rhs: &Expression<F>) -> Option<Expression<F>> {
    fn patterns_inner<F: Field>(lhs: &Expression<F>, rhs: &Expression<F>) -> Option<Expression<F>> {
        if let Some(f) = expr_as_const(lhs) {
            if f == F::ZERO {
                return Some(rhs.clone());
            }
        }
        None
    }

    patterns_inner(lhs, rhs).or_else(|| patterns_inner(rhs, lhs))
}

fn product_patterns<F: Field>(lhs: &Expression<F>, rhs: &Expression<F>) -> Option<Expression<F>> {
    fn patterns_inner<F: Field>(lhs: &Expression<F>, rhs: &Expression<F>) -> Option<Expression<F>> {
        if let Some(f) = expr_as_const(lhs) {
            if f == F::ZERO {
                return Some(Expression::Constant(F::ZERO));
            }
            if f == F::ONE {
                return Some(rhs.clone());
            }
            if f == -F::ONE {
                return Some(Expression::Negated(Box::new(rhs.clone())));
            }
        }
        None
    }

    patterns_inner(lhs, rhs).or_else(|| patterns_inner(rhs, lhs))
}

fn neg_patterns<F: Field>(e: &Expression<F>) -> Option<Expression<F>> {
    match e {
        // Remove double negation
        Expression::Negated(inner) => {
            return Some(inner.as_ref().clone());
        }
        _ => {}
    }
    None
}

impl<F: Field> ExpressionRewriter<F> for ConstantFolding<'_, F> {
    fn on_selector(&self, sel: &Selector) -> Option<Expression<F>> {
        let r = self.resolvers.selector_resolver();
        let resolved = r.resolve_selector(sel).ok()?;
        match resolved {
            ResolvedSelector::Const(bool) => Some(bool.to_f::<F>()),
            ResolvedSelector::Arg(_) => None,
        }
        .map(Expression::Constant)
        .inspect(|e| log::debug!("Folded selector {sel:?} to expression {:?}", ExprDebug(e)))
    }

    fn on_fixed(&self, fixed: &FixedQuery) -> Option<Expression<F>> {
        let r = self.resolvers.query_resolver();
        let resolved = r.resolve_fixed_query(fixed).ok()?;
        match resolved {
            ResolvedQuery::Lit(f) => Some(Expression::Constant(f)),
            ResolvedQuery::IO(_) => None,
        }
        .inspect(|e| {
            log::debug!(
                "Folded fixed query {fixed:?} to expression {:?}",
                ExprDebug(e)
            )
        })
    }

    fn on_negated(&self, e: &Expression<F>) -> Option<Expression<F>> {
        expr_as_const(e)
            .map(|f| -f)
            .map(Expression::Constant)
            .or_else(|| neg_patterns(&e))
            .inspect(|folded| {
                log::debug!(
                    "Folded Negated({:?}) to expression {:?}",
                    ExprDebug(e),
                    ExprDebug(folded)
                )
            })
    }

    fn on_sum(&self, lhs: &Expression<F>, rhs: &Expression<F>) -> Option<Expression<F>> {
        expr_as_const(lhs)
            .zip(expr_as_const(rhs))
            .map(|(lhs, rhs)| lhs + rhs)
            .map(Expression::Constant)
            .or_else(|| sum_patterns(lhs, rhs))
            .inspect(|folded| {
                log::debug!(
                    "Folded Sum({:?}, {:?}) to expression {:?}",
                    ExprDebug(lhs),
                    ExprDebug(rhs),
                    ExprDebug(folded)
                )
            })
    }

    fn on_product(&self, lhs: &Expression<F>, rhs: &Expression<F>) -> Option<Expression<F>> {
        expr_as_const(lhs)
            .zip(expr_as_const(rhs))
            .map(|(lhs, rhs)| lhs * rhs)
            .map(Expression::Constant)
            .or_else(|| product_patterns(lhs, rhs))
            .inspect(|folded| {
                log::debug!(
                    "Folded Product({:?}, {:?}) to expression {:?}",
                    ExprDebug(lhs),
                    ExprDebug(rhs),
                    ExprDebug(folded)
                )
            })
    }

    fn on_scaled(&self, lhs: &Expression<F>, rhs: &F) -> Option<Expression<F>> {
        expr_as_const(lhs)
            .map(|lhs| lhs * *rhs)
            .map(Expression::Constant)
            .or_else(|| {
                // TODO
                None
            })
            .inspect(|folded| {
                log::debug!(
                    "Folded Scaled({:?}, _) to expression {:?}",
                    ExprDebug(lhs),
                    ExprDebug(folded)
                )
            })
    }
}
