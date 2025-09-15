use crate::{
    backend::{
        func::FuncIO,
        lowering::{lowerable::LowerableExpr, ExprLowering},
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    expressions::ScopedExpression,
    halo2::{Challenge, Expression, Field, PrimeField},
    ir::equivalency::{EqvRelation, SymbolicEqv},
};
use anyhow::Result;
use internment::Intern;
use num_bigint::BigUint;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Felt(Intern<BigUint>);

impl Felt {
    pub fn new<F: PrimeField>(f: F) -> Self {
        Self(Intern::new(BigUint::from_bytes_le(f.to_repr().as_ref())))
    }

    pub fn prime<F: PrimeField>() -> Self {
        let f = -F::ONE;
        Self(Intern::new(
            BigUint::from_bytes_le(f.to_repr().as_ref()) + 1usize,
        ))
    }
}

impl From<Felt> for picus::felt::Felt {
    fn from(value: Felt) -> Self {
        Self::new(value.0.as_ref().clone())
    }
}

impl AsRef<BigUint> for Felt {
    fn as_ref(&self) -> &BigUint {
        self.0.as_ref()
    }
}

/// Represents an arithmetic expression.
#[derive(PartialEq, Eq, Clone)]
pub enum IRAexpr {
    Constant(Felt),
    IO(FuncIO),
    Challenge(Challenge),
    Negated(Box<IRAexpr>),
    Sum(Box<IRAexpr>, Box<IRAexpr>),
    Product(Box<IRAexpr>, Box<IRAexpr>),
}

impl IRAexpr {
    fn new<F: PrimeField>(
        expr: &Expression<F>,
        sr: &dyn SelectorResolver,
        qr: &dyn QueryResolver<F>,
    ) -> Result<Self> {
        Ok(match expr {
            Expression::Constant(f) => Self::Constant(Felt::new(*f)),
            Expression::Selector(selector) => match sr.resolve_selector(selector)? {
                ResolvedSelector::Const(bool) => Self::Constant(Felt::new::<F>(bool.to_f())),
                ResolvedSelector::Arg(arg) => Self::IO(arg.into()),
            },
            Expression::Fixed(fixed_query) => match qr.resolve_fixed_query(fixed_query)? {
                ResolvedQuery::IO(io) => Self::IO(io),
                ResolvedQuery::Lit(f) => Self::Constant(Felt::new(f)),
            },
            Expression::Advice(advice_query) => match qr.resolve_advice_query(advice_query)?.0 {
                ResolvedQuery::IO(io) => Self::IO(io),
                ResolvedQuery::Lit(f) => Self::Constant(Felt::new(f)),
            },
            Expression::Instance(instance_query) => {
                match qr.resolve_instance_query(instance_query)? {
                    ResolvedQuery::IO(io) => Self::IO(io),
                    ResolvedQuery::Lit(f) => Self::Constant(Felt::new(f)),
                }
            }
            Expression::Challenge(challenge) => Self::Challenge(*challenge),
            Expression::Negated(expr) => Self::Negated(Box::new(Self::new(expr, sr, qr)?)),
            Expression::Sum(lhs, rhs) => Self::Sum(
                Box::new(Self::new(lhs, sr, qr)?),
                Box::new(Self::new(rhs, sr, qr)?),
            ),
            Expression::Product(lhs, rhs) => Self::Product(
                Box::new(Self::new(lhs, sr, qr)?),
                Box::new(Self::new(rhs, sr, qr)?),
            ),
            Expression::Scaled(lhs, rhs) => Self::Product(
                Box::new(Self::new(lhs, sr, qr)?),
                Box::new(Self::Constant(Felt::new(*rhs))),
            ),
        })
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
            Self::Constant(arg0) => write!(f, "(const {arg0:?})"),
            Self::IO(arg0) => write!(f, "(io {arg0:?})"),
            Self::Challenge(arg0) => write!(f, "(chall {arg0:?})"),
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
            (IRAexpr::Challenge(lhs), IRAexpr::Challenge(rhs)) => lhs == rhs,
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

impl<F> TryFrom<ScopedExpression<'_, '_, F>> for IRAexpr
where
    F: PrimeField,
{
    type Error = anyhow::Error;

    fn try_from(expr: ScopedExpression<'_, '_, F>) -> Result<Self, Self::Error> {
        Self::new(
            expr.as_ref(),
            expr.selector_resolver(),
            expr.query_resolver(),
        )
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
            IRAexpr::Challenge(challenge) => l.lower_challenge(&challenge),
            IRAexpr::Negated(expr) => l.lower_neg(&expr.lower(l)?),
            IRAexpr::Sum(lhs, rhs) => l.lower_sum(&lhs.lower(l)?, &rhs.lower(l)?),
            IRAexpr::Product(lhs, rhs) => l.lower_product(&lhs.lower(l)?, &rhs.lower(l)?),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::resolvers::FixedQueryResolver;
    use crate::expressions::ScopedExpression;
    use crate::ir::equivalency::{EqvRelation as _, SymbolicEqv};
    use crate::synthesis::regions::{RegionRow, Regions};
    use crate::CircuitIO;
    use crate::{halo2::*, synthesis::regions::RegionData};
    use rstest::{fixture, rstest};
    type F = Fr;

    #[allow(dead_code)]
    struct MulCfg {
        cs: ConstraintSystem<F>,
        advices: [Column<Advice>; 3],
        selector: Selector,
    }

    #[fixture]
    fn cs() -> ConstraintSystem<F> {
        ConstraintSystem::default()
    }

    #[fixture]
    fn mul_gate(mut cs: ConstraintSystem<F>) -> MulCfg {
        let col_a = cs.advice_column();
        let col_b = cs.advice_column();
        let col_c = cs.advice_column();
        let selector = cs.selector();

        cs.create_gate("mul", |meta| {
            let s = meta.query_selector(selector);
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());

            vec![s * (a * b - c)]
        });
        MulCfg {
            cs,
            advices: [col_a, col_b, col_c],
            selector,
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
                r.enable_selector(cfg.selector, n);
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
    fn lower_exprs(
        poly: &Expression<F>,
        region: RegionData,
    ) -> anyhow::Result<Vec<super::IRAexpr>> {
        let advice_io = CircuitIO::<Advice>::empty();
        let instance_io = CircuitIO::<Instance>::empty();
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
        for gate in mul_gate.cs.gates() {
            for poly in gate.polynomials() {
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
    }

    /// Dummy resolver that always resolves to zero.
    struct ZeroResolver {}

    impl FixedQueryResolver<F> for ZeroResolver {
        fn resolve_query(&self, _query: &FixedQuery, _row: usize) -> anyhow::Result<F> {
            Ok(F::ZERO)
        }
    }
}
