//! Opaque module that exposes the correct halo2 library based on the implementation selected via
//! feature flags.

//#[cfg(not(feature = "midnight"))]
//pub use halo2curves::bn256;

//#[cfg(feature = "axiom")]
//mod axiom;
//#[cfg(feature = "midnight")]
mod midnight;
//#[cfg(feature = "pse")]
//mod pse;
//#[cfg(feature = "pse-v1")]
//mod pse_v1;
//#[cfg(feature = "scroll")]
//mod scroll;
//#[cfg(feature = "zcash")]
//mod zcash;

//#[cfg(feature = "axiom")]
//pub use axiom::*;
//#[cfg(feature = "midnight")]
pub use midnight::*;

//#[cfg(feature = "pse")]
//pub use pse::*;
//#[cfg(feature = "pse-v1")]
//pub use pse_v1::*;
//#[cfg(feature = "scroll")]
//pub use scroll::*;
//#[cfg(feature = "zcash")]
//pub use zcash::*;

use crate::{
    expressions::{EvalExpression, EvaluableExpr, ExprBuilder, ExpressionInfo},
    info_traits::{ConstraintSystemInfo, GateInfo},
    lookups::LookupData,
};

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

/// Temporary implementation of [`ExpressionInfo`].
impl<F> ExpressionInfo for halo2_proofs::plonk::Expression<F> {
    fn as_negation(&self) -> Option<&Self> {
        match self {
            Self::Negated(expr) => Some(&**expr),
            _ => None,
        }
    }

    fn as_fixed_query(&self) -> Option<&crate::halo2::FixedQuery> {
        match self {
            Self::Fixed(query) => Some(query),
            _ => None,
        }
    }
}

/// Temporary implementation of [`EvaluableExpr`].
impl<F: Field> EvaluableExpr<F> for halo2_proofs::plonk::Expression<F> {
    fn evaluate<E: EvalExpression<F>>(&self, evaluator: &E) -> E::Output {
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

    fn selector(selector: crate::halo2::Selector) -> Self {
        Self::Selector(selector)
    }

    fn fixed(fixed_query: crate::halo2::FixedQuery) -> Self {
        Self::Fixed(fixed_query)
    }

    fn advice(advice_query: crate::halo2::AdviceQuery) -> Self {
        Self::Advice(advice_query)
    }

    fn instance(instance_query: crate::halo2::InstanceQuery) -> Self {
        Self::Instance(instance_query)
    }

    fn challenge(challenge: crate::halo2::Challenge) -> Self {
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

    fn from_column<C: crate::halo2::ColumnType>(
        c: crate::halo2::Column<C>,
        rot: crate::halo2::Rotation,
    ) -> Self {
        c.query_cell(rot)
    }
}
