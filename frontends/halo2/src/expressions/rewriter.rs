use std::ops::Add;

use crate::halo2::{
    AdviceQuery, Challenge, Expression, Field, FixedQuery, InstanceQuery, Selector,
};

use crate::expressions::utils::ExprDebug;

pub trait ExpressionRewriter<F> {
    fn on_constant(&self, f: &F) -> Option<Expression<F>> {
        None
    }

    fn on_selector(&self, sel: &Selector) -> Option<Expression<F>> {
        None
    }

    fn on_fixed(&self, fixed: &FixedQuery) -> Option<Expression<F>> {
        None
    }
    fn on_advice(&self, advice: &AdviceQuery) -> Option<Expression<F>> {
        None
    }
    fn on_instance(&self, instance: &InstanceQuery) -> Option<Expression<F>> {
        None
    }
    fn on_challened(&self, challenge: &Challenge) -> Option<Expression<F>> {
        None
    }
    fn on_negated(&self, e: &Expression<F>) -> Option<Expression<F>> {
        None
    }
    fn on_sum(&self, lhs: &Expression<F>, rhs: &Expression<F>) -> Option<Expression<F>> {
        None
    }
    fn on_product(&self, lhs: &Expression<F>, rhs: &Expression<F>) -> Option<Expression<F>> {
        None
    }
    fn on_scaled(&self, lhs: &Expression<F>, rhs: &F) -> Option<Expression<F>> {
        None
    }
}

fn test_patterns_impl<F: Field>(
    iter_count: usize,
    patterns: &[&dyn ExpressionRewriter<F>],
    fm: impl Fn(&dyn ExpressionRewriter<F>) -> Option<Expression<F>>,
) -> Option<Expression<F>> {
    patterns.iter().find_map(|er| fm(*er)).map(|e| {
        log::debug!(" <== {iter_count} | {:?}", ExprDebug(&e));

        if iter_count == 0 {
            e
        } else {
            rewrite_expr_inner(&e, patterns, iter_count - 1)
        }
    })
}

macro_rules! test_patterns {
    ($method:ident, $iter:ident,$patterns:expr, $ctor:ident,  $($args:expr),* $(,)?) => {{
        log::debug!(" --- {} | {}", $iter, stringify!($method));
       test_patterns_impl($iter, $patterns, |er| { er.$method($(&$args,)+) }).unwrap_or_else( || { Expression::$ctor($($args),+) })
    }};
}

trait Ctors<F> {
    fn negated(e: Self) -> Self;
    fn sum(lhs: Self, rhs: Self) -> Self;
    fn product(lhs: Self, rhs: Self) -> Self;
    fn scaled(lhs: Self, rhs: F) -> Self;
}

impl<F> Ctors<F> for Expression<F> {
    fn negated(e: Self) -> Self {
        Self::Negated(Box::new(e))
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

fn rewrite_expr_inner<F>(
    r: &Expression<F>,
    patterns: &[&dyn ExpressionRewriter<F>],
    iter_count: usize,
) -> Expression<F>
where
    F: Field,
{
    log::debug!(" ==> {iter_count} | {:?}", ExprDebug(r));
    r.evaluate(
        &|f| test_patterns!(on_constant, iter_count, patterns, Constant, f),
        &|sel| test_patterns!(on_selector, iter_count, patterns, Selector, sel),
        &|fixed| test_patterns!(on_fixed, iter_count, patterns, Fixed, fixed),
        &|advice| test_patterns!(on_advice, iter_count, patterns, Advice, advice),
        &|instance| test_patterns!(on_instance, iter_count, patterns, Instance, instance),
        &|challenge| test_patterns!(on_challened, iter_count, patterns, Challenge, challenge),
        &|e| test_patterns!(on_negated, iter_count, patterns, negated, e),
        &|lhs, rhs| test_patterns!(on_sum, iter_count, patterns, sum, lhs, rhs),
        &|lhs, rhs| test_patterns!(on_product, iter_count, patterns, product, lhs, rhs),
        &|lhs, rhs| test_patterns!(on_scaled, iter_count, patterns, scaled, lhs, rhs),
    )
}

const MAX_ITERS: usize = 20;

/// Rewrites the expression without recursing on the newly generated expressions
pub fn rewrite_expr<F>(r: &Expression<F>, patterns: &[&dyn ExpressionRewriter<F>]) -> Expression<F>
where
    F: Field,
{
    rewrite_expr_inner(r, patterns, 0)
}

/// Rewrites the expression and recurses on the newly generated expressions to try match more
/// patterns.
pub fn rewrite_rec_expr<F>(
    r: &Expression<F>,
    patterns: &[&dyn ExpressionRewriter<F>],
) -> Expression<F>
where
    F: Field,
{
    rewrite_expr_inner(r, patterns, MAX_ITERS)
}
