use crate::{
    backend::{
        func::FuncIO,
        lowering::{lowerable::LowerableExpr, ExprLowering},
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    expressions::ScopedExpression,
    halo2::{Challenge, Expression, Field},
    ir::equivalency::{EqvRelation, SymbolicEqv},
};
use anyhow::Result;

/// Represents an arithmetic expression.
pub enum IRAexpr<F> {
    Constant(F),
    IO(FuncIO),
    Challenge(Challenge),
    Negated(Box<IRAexpr<F>>),
    Sum(Box<IRAexpr<F>>, Box<IRAexpr<F>>),
    Product(Box<IRAexpr<F>>, Box<IRAexpr<F>>),
}

impl<F> IRAexpr<F> {
    fn new(
        expr: &Expression<F>,
        sr: &dyn SelectorResolver,
        qr: &dyn QueryResolver<F>,
    ) -> Result<Self>
    where
        F: Field,
    {
        Ok(match expr {
            Expression::Constant(f) => Self::Constant(*f),
            Expression::Selector(selector) => match sr.resolve_selector(selector)? {
                ResolvedSelector::Const(bool) => Self::Constant(bool.to_f()),
                ResolvedSelector::Arg(arg) => Self::IO(arg.into()),
            },
            Expression::Fixed(fixed_query) => match qr.resolve_fixed_query(fixed_query)? {
                ResolvedQuery::IO(io) => Self::IO(io),
                ResolvedQuery::Lit(f) => Self::Constant(f),
            },
            Expression::Advice(advice_query) => match qr.resolve_advice_query(advice_query)?.0 {
                ResolvedQuery::IO(io) => Self::IO(io),
                ResolvedQuery::Lit(f) => Self::Constant(f),
            },
            Expression::Instance(instance_query) => {
                match qr.resolve_instance_query(instance_query)? {
                    ResolvedQuery::IO(io) => Self::IO(io),
                    ResolvedQuery::Lit(f) => Self::Constant(f),
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
                Box::new(Self::Constant(*rhs)),
            ),
        })
    }

    pub fn map<O>(self, f: &impl Fn(F) -> O) -> IRAexpr<O> {
        match self {
            IRAexpr::Constant(felt) => IRAexpr::Constant(f(felt)),
            IRAexpr::IO(func_io) => IRAexpr::IO(func_io),
            IRAexpr::Challenge(challenge) => IRAexpr::Challenge(challenge),
            IRAexpr::Negated(expr) => IRAexpr::Negated(Box::new(expr.map(f))),
            IRAexpr::Sum(lhs, rhs) => IRAexpr::Sum(Box::new(lhs.map(f)), Box::new(rhs.map(f))),
            IRAexpr::Product(lhs, rhs) => {
                IRAexpr::Product(Box::new(lhs.map(f)), Box::new(rhs.map(f)))
            }
        }
    }

    pub fn try_map<O>(self, f: &impl Fn(F) -> Result<O>) -> Result<IRAexpr<O>> {
        Ok(match self {
            IRAexpr::Constant(felt) => IRAexpr::Constant(f(felt)?),
            IRAexpr::IO(func_io) => IRAexpr::IO(func_io),
            IRAexpr::Challenge(challenge) => IRAexpr::Challenge(challenge),
            IRAexpr::Negated(expr) => IRAexpr::Negated(Box::new(expr.try_map(f)?)),
            IRAexpr::Sum(lhs, rhs) => {
                IRAexpr::Sum(Box::new(lhs.try_map(f)?), Box::new(rhs.try_map(f)?))
            }
            IRAexpr::Product(lhs, rhs) => {
                IRAexpr::Product(Box::new(lhs.try_map(f)?), Box::new(rhs.try_map(f)?))
            }
        })
    }

    /// Similar to [`AExpr::try_map`] but maps the IO instead.
    pub fn try_map_io(self, f: &impl Fn(FuncIO) -> Result<FuncIO>) -> Result<Self> {
        Ok(match self {
            IRAexpr::Constant(felt) => IRAexpr::Constant(felt),
            IRAexpr::IO(func_io) => IRAexpr::IO(f(func_io)?),
            IRAexpr::Challenge(challenge) => IRAexpr::Challenge(challenge),
            IRAexpr::Negated(expr) => IRAexpr::Negated(Box::new(expr.try_map_io(f)?)),
            IRAexpr::Sum(lhs, rhs) => {
                IRAexpr::Sum(Box::new(lhs.try_map_io(f)?), Box::new(rhs.try_map_io(f)?))
            }
            IRAexpr::Product(lhs, rhs) => {
                IRAexpr::Product(Box::new(lhs.try_map_io(f)?), Box::new(rhs.try_map_io(f)?))
            }
        })
    }
}

impl<F: std::fmt::Debug> std::fmt::Debug for IRAexpr<F> {
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

impl<F: PartialEq> EqvRelation<IRAexpr<F>> for SymbolicEqv {
    /// Two arithmetic expressions are equivalent if they are structurally equal, constant values
    /// equal and variables are equivalent.
    fn equivalent(lhs: &IRAexpr<F>, rhs: &IRAexpr<F>) -> bool {
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

impl<F> TryFrom<ScopedExpression<'_, '_, F>> for IRAexpr<F>
where
    F: Field,
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

impl<F: PartialEq> PartialEq for IRAexpr<F> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (IRAexpr::Constant(lhs), IRAexpr::Constant(rhs)) => lhs == rhs,
            (IRAexpr::IO(lhs), IRAexpr::IO(rhs)) => lhs == rhs,
            (IRAexpr::Challenge(lhs), IRAexpr::Challenge(rhs)) => lhs == rhs,
            (IRAexpr::Negated(lhs), IRAexpr::Negated(rhs)) => lhs == rhs,
            (IRAexpr::Sum(lhs0, lhs1), IRAexpr::Sum(rhs0, rhs1)) => lhs0 == rhs0 && lhs1 == rhs1,
            (IRAexpr::Product(lhs0, lhs1), IRAexpr::Product(rhs0, rhs1)) => {
                lhs0 == rhs0 && lhs1 == rhs1
            }
            _ => false,
        }
    }
}

impl<F> LowerableExpr for IRAexpr<F>
where
    F: Field,
{
    type F = F;

    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering<F = Self::F> + ?Sized,
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
    ) -> anyhow::Result<Vec<super::IRAexpr<F>>> {
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
