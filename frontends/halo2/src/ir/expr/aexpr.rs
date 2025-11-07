//! Structs for handling arithmetic expressions.

use std::{
    marker::PhantomData,
    ops::{Add, Deref, Mul, Rem, RemAssign, Sub},
};

use crate::{
    backend::{
        func::FuncIO,
        lowering::{ExprLowering, lowerable::LowerableExpr},
    },
    expressions::ScopedExpression,
    ir::equivalency::{EqvRelation, SymbolicEqv},
    resolvers::{
        ChallengeResolver, QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver,
    },
    temps::ExprOrTemp,
};
use halo2_frontend_core::expressions::{EvalExpression, EvaluableExpr, ExpressionTypes};

use anyhow::Result;
use ff::PrimeField;
use internment::Intern;
use num_bigint::BigUint;

/// Represents a constant value.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Felt(Intern<BigUint>);

impl Felt {
    /// Creates a new felt from an implementation of [`PrimeField`].
    pub fn new<F: PrimeField>(f: F) -> Self {
        Self(Intern::new(BigUint::from_bytes_le(f.to_repr().as_ref())))
    }

    /// Creates a new felt whose value is the prime in the [`PrimeField`].
    pub fn prime<F: PrimeField>() -> Self {
        let f = -F::ONE;
        Self(Intern::new(
            BigUint::from_bytes_le(f.to_repr().as_ref()) + 1usize,
        ))
    }
}

impl std::fmt::Debug for Felt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

#[cfg(feature = "picus-backend")]
impl From<Felt> for picus::felt::Felt {
    fn from(value: Felt) -> Self {
        Self::new(value.0.as_ref().clone())
    }
}

impl<T: Into<BigUint>> From<T> for Felt {
    fn from(value: T) -> Self {
        Self(Intern::new(value.into()))
    }
}

impl<T> PartialEq<T> for Felt
where
    T: Into<BigUint> + Copy,
{
    fn eq(&self, other: &T) -> bool {
        self.as_ref().eq(&(*other).into())
    }
}

impl AsRef<BigUint> for Felt {
    fn as_ref(&self) -> &BigUint {
        self.0.as_ref()
    }
}

impl Deref for Felt {
    type Target = BigUint;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Rem for Felt {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        if self < rhs {
            return self;
        }
        ((*self).clone() % (*rhs).clone()).into()
    }
}

impl RemAssign for Felt {
    fn rem_assign(&mut self, rhs: Self) {
        if *self > rhs {
            *self = *self % rhs;
        }
    }
}

impl Sub for Felt {
    type Output = Option<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self < rhs {
            return None;
        }

        Some(((*self).clone() - (*rhs).clone()).into())
    }
}

impl Add for Felt {
    type Output = Felt;

    fn add(self, rhs: Self) -> Self::Output {
        ((*self).clone() + (*rhs).clone()).into()
    }
}

impl Mul for Felt {
    type Output = Felt;

    fn mul(self, rhs: Self) -> Self::Output {
        ((*self).clone() * (*rhs).clone()).into()
    }
}

impl std::fmt::Display for Felt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

/// Represents an arithmetic expression.
#[derive(PartialEq, Eq, Clone)]
pub enum IRAexpr {
    /// Constant value.
    Constant(Felt),
    /// IO element of the circuit; inputs, outputs, cells, etc.
    IO(FuncIO),
    /// Represents the negation of the inner expression.
    Negated(Box<IRAexpr>),
    /// Represents the sum of the inner expressions.
    Sum(Box<IRAexpr>, Box<IRAexpr>),
    /// Represents the product of the inner expresions.
    Product(Box<IRAexpr>, Box<IRAexpr>),
}

impl IRAexpr {
    fn new<F, E>(
        expr: &E,
        sr: &dyn SelectorResolver,
        qr: &dyn QueryResolver<F>,
        cr: &dyn ChallengeResolver,
    ) -> Result<Self>
    where
        F: PrimeField,
        E: EvaluableExpr<F>,
    {
        expr.evaluate(&PolyToAexpr::new(sr, qr, cr))
    }

    /// Returns `Some(_)` if the expression is a constant value. None otherwise.
    pub fn const_value(&self) -> Option<Felt> {
        match self {
            IRAexpr::Constant(f) => Some(*f),
            _ => None,
        }
    }

    /// Folds the expression if the values are constant.
    pub(crate) fn constant_fold(&mut self, prime: Felt) {
        match self {
            IRAexpr::Constant(felt) => *felt %= prime,
            IRAexpr::IO(_) => {}
            IRAexpr::Negated(expr) => {
                expr.constant_fold(prime);
                if let Some(f) = expr.const_value().and_then(|f| prime - f) {
                    *self = IRAexpr::Constant(f % prime);
                }
            }

            IRAexpr::Sum(lhs, rhs) => {
                lhs.constant_fold(prime);
                rhs.constant_fold(prime);

                match (lhs.const_value(), rhs.const_value()) {
                    (Some(lhs), Some(rhs)) => {
                        *self = IRAexpr::Constant((lhs + rhs) % prime);
                    }
                    (None, Some(rhs)) if rhs == 0usize => {
                        *self = (**lhs).clone();
                    }
                    (Some(lhs), None) if lhs == 0usize => {
                        *self = (**rhs).clone();
                    }
                    _ => {}
                }
            }
            IRAexpr::Product(lhs, rhs) => {
                let minus_one = (prime - 1usize.into()).unwrap();
                lhs.constant_fold(prime);
                rhs.constant_fold(prime);
                match (lhs.const_value(), rhs.const_value()) {
                    (Some(lhs), Some(rhs)) => {
                        *self = IRAexpr::Constant((lhs * rhs) % prime);
                    }
                    // (* 1 X) => X
                    (None, Some(rhs)) if rhs == 1usize => {
                        *self = (**lhs).clone();
                    }
                    (Some(lhs), None) if lhs == 1usize => {
                        *self = (**rhs).clone();
                    }
                    // (* 0 X) => X
                    (None, Some(rhs)) if rhs == 0usize => {
                        *self = IRAexpr::Constant(0usize.into());
                    }
                    (Some(lhs), None) if lhs == 0usize => {
                        *self = IRAexpr::Constant(0usize.into());
                    }
                    // (* -1 X) => -X
                    (None, Some(rhs)) if rhs == minus_one => {
                        *self = IRAexpr::Negated(lhs.clone());
                    }
                    (Some(lhs), None) if lhs == minus_one => {
                        *self = IRAexpr::Negated(rhs.clone());
                    }
                    _ => {}
                }
            }
        }
    }

    /// Similar to [`AExpr::try_map`] but maps the IO instead and edits in-place.
    pub fn try_map_io(&mut self, f: &impl Fn(&mut FuncIO) -> Result<()>) -> Result<()> {
        match self {
            IRAexpr::IO(func_io) => f(func_io),
            IRAexpr::Negated(expr) => expr.try_map_io(f),
            IRAexpr::Sum(lhs, rhs) => {
                lhs.try_map_io(f)?;
                rhs.try_map_io(f)
            }
            IRAexpr::Product(lhs, rhs) => {
                lhs.try_map_io(f)?;
                rhs.try_map_io(f)
            }
            _ => Ok(()),
        }
    }
}

impl std::fmt::Debug for IRAexpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant(arg0) => write!(f, "{arg0:?}"),
            Self::IO(arg0) => write!(f, "{arg0:?}"),
            Self::Negated(arg0) => write!(f, "(- {arg0:?})"),
            Self::Sum(arg0, arg1) => write!(f, "(+ {arg0:?} {arg1:?})"),
            Self::Product(arg0, arg1) => write!(f, "(* {arg0:?} {arg1:?})"),
        }
    }
}

impl EqvRelation<IRAexpr> for SymbolicEqv {
    /// Two arithmetic expressions are equivalent if they are structurally equal, constant values
    /// equal and variables are equivalent.
    fn equivalent(lhs: &IRAexpr, rhs: &IRAexpr) -> bool {
        match (lhs, rhs) {
            (IRAexpr::Constant(lhs), IRAexpr::Constant(rhs)) => lhs == rhs,
            (IRAexpr::IO(lhs), IRAexpr::IO(rhs)) => Self::equivalent(lhs, rhs),
            (IRAexpr::Negated(lhs), IRAexpr::Negated(rhs)) => Self::equivalent(lhs, rhs),
            (IRAexpr::Sum(lhs0, lhs1), IRAexpr::Sum(rhs0, rhs1)) => {
                Self::equivalent(lhs0, rhs0) && Self::equivalent(lhs1, rhs1)
            }
            (IRAexpr::Product(lhs0, lhs1), IRAexpr::Product(rhs0, rhs1)) => {
                Self::equivalent(lhs0, rhs0) && Self::equivalent(lhs1, rhs1)
            }
            _ => false,
        }
    }
}

impl<F, E> TryFrom<ScopedExpression<'_, '_, F, E>> for IRAexpr
where
    F: PrimeField,
    E: EvaluableExpr<F> + Clone,
{
    type Error = anyhow::Error;

    fn try_from(expr: ScopedExpression<'_, '_, F, E>) -> Result<Self, Self::Error> {
        Self::new(
            expr.as_ref(),
            expr.selector_resolver(),
            expr.query_resolver(),
            expr.challenge_resolver(),
        )
    }
}

impl<E> TryFrom<ExprOrTemp<E>> for IRAexpr
where
    IRAexpr: TryFrom<E>,
{
    type Error = <E as TryInto<IRAexpr>>::Error;

    fn try_from(value: ExprOrTemp<E>) -> std::result::Result<Self, Self::Error> {
        match value {
            ExprOrTemp::Temp(temp) => Ok(IRAexpr::IO(temp.into())),
            ExprOrTemp::Expr(e) => e.try_into(),
        }
    }
}

impl LowerableExpr for IRAexpr {
    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering + ?Sized,
    {
        match self {
            IRAexpr::Constant(f) => l.lower_constant(f),
            IRAexpr::IO(io) => l.lower_funcio(io),
            IRAexpr::Negated(expr) => l.lower_neg(&expr.lower(l)?),
            IRAexpr::Sum(lhs, rhs) => l.lower_sum(&lhs.lower(l)?, &rhs.lower(l)?),
            IRAexpr::Product(lhs, rhs) => l.lower_product(&lhs.lower(l)?, &rhs.lower(l)?),
        }
    }
}

/// Implements the conversion logic between an expression and [`IRAexpr`].
struct PolyToAexpr<'r, F, E> {
    sr: &'r dyn SelectorResolver,
    qr: &'r dyn QueryResolver<F>,
    cr: &'r dyn ChallengeResolver,
    _marker: PhantomData<E>,
}

impl<'r, F, E> PolyToAexpr<'r, F, E> {
    pub fn new(
        sr: &'r dyn SelectorResolver,
        qr: &'r dyn QueryResolver<F>,
        cr: &'r dyn ChallengeResolver,
    ) -> Self {
        Self {
            sr,
            qr,
            cr,
            _marker: Default::default(),
        }
    }
}

impl<F: PrimeField, E: ExpressionTypes> EvalExpression<F, E> for PolyToAexpr<'_, F, E> {
    type Output = Result<IRAexpr>;

    fn constant(&self, f: &F) -> Self::Output {
        Ok(IRAexpr::Constant(Felt::new(*f)))
    }

    fn selector(&self, selector: &E::Selector) -> Self::Output {
        Ok(match self.sr.resolve_selector(selector)? {
            ResolvedSelector::Const(bool) => IRAexpr::Constant(Felt::new::<F>(bool.to_f())),
            ResolvedSelector::Arg(arg) => IRAexpr::IO(arg.into()),
        })
    }

    fn fixed(&self, fixed_query: &E::FixedQuery) -> Self::Output {
        Ok(match self.qr.resolve_fixed_query(fixed_query)? {
            ResolvedQuery::IO(io) => IRAexpr::IO(io),
            ResolvedQuery::Lit(f) => IRAexpr::Constant(Felt::new(f)),
        })
    }

    fn advice(&self, advice_query: &E::AdviceQuery) -> Self::Output {
        Ok(match self.qr.resolve_advice_query(advice_query)? {
            ResolvedQuery::IO(io) => IRAexpr::IO(io),
            ResolvedQuery::Lit(f) => IRAexpr::Constant(Felt::new(f)),
        })
    }

    fn instance(&self, instance_query: &E::InstanceQuery) -> Self::Output {
        Ok(match self.qr.resolve_instance_query(instance_query)? {
            ResolvedQuery::IO(io) => IRAexpr::IO(io),
            ResolvedQuery::Lit(f) => IRAexpr::Constant(Felt::new(f)),
        })
    }

    fn challenge(&self, challenge: &E::Challenge) -> Self::Output {
        Ok(IRAexpr::IO(self.cr.resolve_challenge(challenge)?))
    }

    fn negated(&self, expr: Self::Output) -> Self::Output {
        Ok(IRAexpr::Negated(Box::new(expr?)))
    }

    fn sum(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output {
        Ok(IRAexpr::Sum(Box::new(lhs?), Box::new(rhs?)))
    }

    fn product(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output {
        Ok(IRAexpr::Product(Box::new(lhs?), Box::new(rhs?)))
    }

    fn scaled(&self, lhs: Self::Output, rhs: &F) -> Self::Output {
        Ok(IRAexpr::Product(
            Box::new(lhs?),
            Box::new(self.constant(rhs)?),
        ))
    }
}

#[cfg(test)]
mod folding_tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn seven() -> Felt {
        Felt::from(7usize)
    }

    #[rstest]
    fn folding_constant_within_field(seven: Felt) {
        let mut test = IRAexpr::Constant(5usize.into());
        let expected = test.clone();
        test.constant_fold(seven);
        assert_eq!(test, expected);
    }

    #[rstest]
    fn folding_constant_outside_field(seven: Felt) {
        let mut test = IRAexpr::Constant(8usize.into());
        let expected = IRAexpr::Constant(1usize.into());
        test.constant_fold(seven);
        assert_eq!(test, expected);
    }

    #[rstest]
    fn mult_identity(seven: Felt) {
        let lhs = IRAexpr::Constant(1usize.into());
        let rhs = IRAexpr::IO(FuncIO::Arg(0.into()));
        let mut mul = IRAexpr::Product(Box::new(lhs), Box::new(rhs.clone()));
        mul.constant_fold(seven);
        assert_eq!(mul, rhs);
    }

    #[rstest]
    fn mult_identity_rev(seven: Felt) {
        let rhs = IRAexpr::Constant(1usize.into());
        let lhs = IRAexpr::IO(FuncIO::Arg(0.into()));
        let mut mul = IRAexpr::Product(Box::new(lhs.clone()), Box::new(rhs));
        mul.constant_fold(seven);
        assert_eq!(mul, lhs);
    }

    #[rstest]
    fn mult_by_zero(seven: Felt) {
        let lhs = IRAexpr::Constant(0usize.into());
        let rhs = IRAexpr::IO(FuncIO::Arg(0.into()));
        let mut mul = IRAexpr::Product(Box::new(lhs.clone()), Box::new(rhs));
        mul.constant_fold(seven);
        assert_eq!(mul, lhs);
    }

    #[rstest]
    fn mult_by_zero_rev(seven: Felt) {
        let rhs = IRAexpr::Constant(0usize.into());
        let lhs = IRAexpr::IO(FuncIO::Arg(0.into()));
        let mut mul = IRAexpr::Product(Box::new(lhs), Box::new(rhs.clone()));
        mul.constant_fold(seven);
        assert_eq!(mul, rhs);
    }

    #[rstest]
    fn sum_identity(seven: Felt) {
        let lhs = IRAexpr::Constant(0usize.into());
        let rhs = IRAexpr::IO(FuncIO::Arg(0.into()));
        let mut sum = IRAexpr::Sum(Box::new(lhs), Box::new(rhs.clone()));
        sum.constant_fold(seven);
        assert_eq!(sum, rhs);
    }

    #[rstest]
    fn sum_identity_rev(seven: Felt) {
        let rhs = IRAexpr::Constant(0usize.into());
        let lhs = IRAexpr::IO(FuncIO::Arg(0.into()));
        let mut sum = IRAexpr::Sum(Box::new(lhs.clone()), Box::new(rhs));
        sum.constant_fold(seven);
        assert_eq!(sum, lhs);
    }
}

#[cfg(test)]
mod lowering_tests {
    use crate::CircuitIO;
    use crate::expressions::ScopedExpression;
    use crate::info_traits::QueryInfo;
    use crate::ir::equivalency::{EqvRelation as _, SymbolicEqv};
    use crate::ir::expr::aexpr::lowering_tests::mocks::{Expr, Selector};
    use crate::resolvers::{Advice, Fixed, FixedQueryResolver};
    use crate::synthesis::regions::{RegionData, RegionIndex};
    use crate::synthesis::regions::{RegionRow, Regions};
    use crate::table::Column;
    use ff::Field;
    use rstest::{fixture, rstest};

    type F = halo2curves::bn256::Fr;

    struct MulCfg {
        advices: [Column<Advice>; 3],
        selector: Selector,
        gates: Vec<Expr>,
    }

    #[fixture]
    fn mul_gate() -> MulCfg {
        let col_a = Column::new(0, Advice);
        let col_b = Column::new(1, Advice);
        let col_c = Column::new(2, Advice);
        let selector = Selector(0);

        let a = Expr::Advice(col_a, 0);
        let b = Expr::Advice(col_b, 0);
        let c = Expr::Advice(col_c, 0);

        let gates = vec![Expr::Selector(selector) * (a * b - c)];
        MulCfg {
            advices: [col_a, col_b, col_c],
            selector,
            gates,
        }
    }

    /// Creates two identical consecutive regions
    fn two_regions(cfg: &MulCfg) -> Regions {
        let mut r = Regions::default();
        let mut indices = (0..).map(RegionIndex::from);
        let mut tables = vec![];
        for n in 0..2 {
            log::debug!("Creating region #{n}");
            r.push(|| "region", &mut indices, &mut tables);
            r.edit(|r| {
                r.enable_selector(&cfg.selector, n);
                // Fake using some cells.
                for col in cfg.advices {
                    r.update_extent(col.into(), n);
                }
            });
            r.commit();
        }

        r
    }

    /// Lowers the expression in the scope of the region.
    /// Returns one expression per row.
    fn lower_exprs(poly: &Expr, region: RegionData) -> anyhow::Result<Vec<super::IRAexpr>> {
        let advice_io = CircuitIO::<crate::Advice>::empty();
        let instance_io = CircuitIO::<crate::Instance>::empty();
        let zero = ZeroResolver {};

        region
            .rows()
            .map(|row| RegionRow::new(region, row, &advice_io, &instance_io, &zero))
            .map(|rr| ScopedExpression::from_ref(poly, rr))
            .map(TryInto::try_into)
            .collect()
    }

    #[rstest]
    fn mul_gate_equivalence(mul_gate: MulCfg) {
        let _ = simplelog::TestLogger::init(log::LevelFilter::Debug, simplelog::Config::default());
        let regions = two_regions(&mul_gate);

        assert_eq!(regions.regions().len(), 2);
        for poly in &mul_gate.gates {
            let exprs0 = lower_exprs(poly, regions.regions()[0]).unwrap();
            log::debug!("expr0:");
            for e in &exprs0 {
                log::debug!("  {e:?}");
            }
            let exprs1 = lower_exprs(poly, regions.regions()[1]).unwrap();
            log::debug!("expr1:");
            for e in &exprs1 {
                log::debug!("  {e:?}");
            }
            assert!(SymbolicEqv::equivalent(&exprs0, &exprs1));
        }
    }

    /// Dummy resolver that always resolves to zero.
    struct ZeroResolver {}

    impl FixedQueryResolver<F> for ZeroResolver {
        fn resolve_query(
            &self,
            _query: &dyn QueryInfo<Kind = Fixed>,
            _row: usize,
        ) -> anyhow::Result<F> {
            Ok(F::ZERO)
        }
    }

    mod mocks {
        use std::ops::{Mul, Sub};

        use ff::Field;

        use crate::{
            Advice, Fixed, Instance,
            expressions::{EvalExpression, EvaluableExpr, ExpressionTypes},
            info_traits::{ChallengeInfo, CreateQuery, QueryInfo, SelectorInfo},
            table::{Column, Rotation},
        };

        #[derive(Copy, Clone, Debug)]
        pub struct Selector(pub usize);

        impl SelectorInfo for Selector {
            fn id(&self) -> usize {
                self.0
            }
        }

        #[derive(Copy, Clone, Debug)]
        pub enum Binop {
            Mul,
            Sub,
        }

        #[derive(Clone, Debug)]
        pub enum Expr {
            Selector(Selector),
            Advice(Column<Advice>, i32),
            Binop(Binop, Box<Expr>, Box<Expr>),
        }

        impl ExpressionTypes for Expr {
            type Selector = Selector;

            type FixedQuery = MockFixedQuery;

            type AdviceQuery = (Column<Advice>, i32);

            type InstanceQuery = MockInstanceQuery;

            type Challenge = ();
        }

        impl<F: Field> EvaluableExpr<F> for Expr {
            fn evaluate<E: EvalExpression<F, Self>>(&self, evaluator: &E) -> E::Output {
                match self {
                    Expr::Selector(selector) => evaluator.selector(selector),
                    Expr::Advice(column, rot) => evaluator.advice(&(*column, *rot)),
                    Expr::Binop(Binop::Mul, expr, expr1) => {
                        evaluator.product(expr.evaluate(evaluator), expr1.evaluate(evaluator))
                    }
                    Expr::Binop(Binop::Sub, expr, expr1) => evaluator.sum(
                        expr.evaluate(evaluator),
                        evaluator.negated(expr1.evaluate(evaluator)),
                    ),
                }
            }
        }

        impl Mul for Expr {
            type Output = Self;

            fn mul(self, rhs: Self) -> Self::Output {
                Expr::Binop(Binop::Mul, Box::new(self), Box::new(rhs))
            }
        }

        impl Sub for Expr {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                Expr::Binop(Binop::Sub, Box::new(self), Box::new(rhs))
            }
        }

        impl QueryInfo for (Column<Advice>, i32) {
            type Kind = Advice;

            fn rotation(&self) -> Rotation {
                self.1
            }

            fn column_index(&self) -> usize {
                self.0.index()
            }
        }

        impl CreateQuery<Expr> for (Column<Advice>, i32) {
            fn query_expr(index: usize, at: Rotation) -> Expr {
                Expr::Advice(Column::new(index, Advice), at)
            }
        }

        #[derive(Copy, Clone, Debug)]
        pub struct MockFixedQuery;

        impl QueryInfo for MockFixedQuery {
            type Kind = Fixed;

            fn rotation(&self) -> Rotation {
                unreachable!()
            }

            fn column_index(&self) -> usize {
                unreachable!()
            }
        }

        impl CreateQuery<Expr> for MockFixedQuery {
            fn query_expr(_index: usize, _at: Rotation) -> Expr {
                unreachable!()
            }
        }

        #[derive(Copy, Clone, Debug)]
        pub struct MockInstanceQuery;

        impl QueryInfo for MockInstanceQuery {
            type Kind = Instance;

            fn rotation(&self) -> Rotation {
                unreachable!()
            }

            fn column_index(&self) -> usize {
                unreachable!()
            }
        }

        impl CreateQuery<Expr> for MockInstanceQuery {
            fn query_expr(_index: usize, _at: Rotation) -> Expr {
                unreachable!()
            }
        }

        impl ChallengeInfo for () {
            fn index(&self) -> usize {
                unreachable!()
            }

            fn phase(&self) -> u8 {
                unreachable!()
            }
        }
    }
}
