use super::{
    equivalency::{EqvRelation, SymbolicEqv},
    CmpOp,
};
use crate::{
    backend::{
        func::FuncIO,
        lowering::{lowerable::LowerableExpr, ExprLowering},
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    expressions::ScopedExpression,
    halo2::{Challenge, Expression, Field},
};
use anyhow::Result;
use std::{
    convert::identity,
    ops::{BitAnd, BitOr, Not},
};

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
            Expression::Negated(expr) => Self::Negated(Box::new(Self::new(&expr, sr, qr)?)),
            Expression::Sum(lhs, rhs) => Self::Sum(
                Box::new(Self::new(&lhs, sr, qr)?),
                Box::new(Self::new(&rhs, sr, qr)?),
            ),
            Expression::Product(lhs, rhs) => Self::Product(
                Box::new(Self::new(&lhs, sr, qr)?),
                Box::new(Self::new(&rhs, sr, qr)?),
            ),
            Expression::Scaled(lhs, rhs) => Self::Product(
                Box::new(Self::new(&lhs, sr, qr)?),
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

/// Represents boolean expressions over some arithmetic expression type A.
pub enum IRBexpr<A> {
    Cmp(CmpOp, A, A),
    And(Vec<IRBexpr<A>>),
    Or(Vec<IRBexpr<A>>),
    Not(Box<IRBexpr<A>>),
}

impl<T> IRBexpr<T> {
    pub fn map<O>(self, f: &impl Fn(T) -> O) -> IRBexpr<O> {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => IRBexpr::Cmp(cmp_op, f(lhs), f(rhs)),
            IRBexpr::And(exprs) => IRBexpr::And(exprs.into_iter().map(|e| e.map(f)).collect()),
            IRBexpr::Or(exprs) => IRBexpr::Or(exprs.into_iter().map(|e| e.map(f)).collect()),
            IRBexpr::Not(expr) => IRBexpr::Not(Box::new(expr.map(f))),
        }
    }

    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<IRBexpr<O>> {
        Ok(match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => IRBexpr::Cmp(cmp_op, f(lhs)?, f(rhs)?),
            IRBexpr::And(exprs) => IRBexpr::And(
                exprs
                    .into_iter()
                    .map(|e| e.try_map(f))
                    .collect::<Result<Vec<_>>>()?,
            ),
            IRBexpr::Or(exprs) => IRBexpr::Or(
                exprs
                    .into_iter()
                    .map(|e| e.try_map(f))
                    .collect::<Result<Vec<_>>>()?,
            ),
            IRBexpr::Not(expr) => IRBexpr::Not(Box::new(expr.try_map(f)?)),
        })
    }
}

/// IRBexpr transilitively inherits any equivalence relation.
impl<L, R, E: EqvRelation<L, R>> EqvRelation<IRBexpr<L>, IRBexpr<R>> for E {
    /// Two boolean expressions are equivalent if they are structurally equal and their inner entities
    /// are equivalent.
    fn equivalent(lhs: &IRBexpr<L>, rhs: &IRBexpr<R>) -> bool {
        match (lhs, rhs) {
            (IRBexpr::Cmp(op1, lhs1, rhs1), IRBexpr::Cmp(op2, lhs2, rhs2)) => {
                op1 == op2 && E::equivalent(lhs1, lhs2) && E::equivalent(rhs1, rhs2)
            }
            (IRBexpr::And(lhs), IRBexpr::And(rhs)) => {
                <E as EqvRelation<Vec<IRBexpr<L>>, Vec<IRBexpr<R>>>>::equivalent(lhs, rhs)
            }
            (IRBexpr::Or(lhs), IRBexpr::Or(rhs)) => {
                <E as EqvRelation<Vec<IRBexpr<L>>, Vec<IRBexpr<R>>>>::equivalent(lhs, rhs)
            }
            (IRBexpr::Not(lhs), IRBexpr::Not(rhs)) => {
                <E as EqvRelation<Box<IRBexpr<L>>, Box<IRBexpr<R>>>>::equivalent(lhs, rhs)
            }
            _ => false,
        }
    }
}

fn concat<L, R, T>(lhs: L, rhs: R) -> Vec<IRBexpr<T>>
where
    L: IntoIterator<Item = IRBexpr<T>>,
    R: IntoIterator<Item = IRBexpr<T>>,
{
    lhs.into_iter().chain(rhs).collect()
}

impl<T> BitAnd for IRBexpr<T> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (IRBexpr::And(lhs), IRBexpr::And(rhs)) => IRBexpr::And(concat(lhs, rhs)),
            (lhs, IRBexpr::And(rhs)) => IRBexpr::And(concat([lhs], rhs)),
            (IRBexpr::And(lhs), rhs) => IRBexpr::And(concat(lhs, [rhs])),
            (lhs, rhs) => IRBexpr::And(vec![lhs, rhs]),
        }
    }
}

impl<T> BitOr for IRBexpr<T> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (IRBexpr::Or(lhs), IRBexpr::Or(rhs)) => IRBexpr::Or(concat(lhs, rhs)),
            (lhs, IRBexpr::Or(rhs)) => IRBexpr::Or(concat([lhs], rhs)),
            (IRBexpr::Or(lhs), rhs) => IRBexpr::Or(concat(lhs, [rhs])),
            (lhs, rhs) => IRBexpr::Or(vec![lhs, rhs]),
        }
    }
}

impl<T> Not for IRBexpr<T> {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            IRBexpr::Not(e) => *e,
            e => IRBexpr::Not(Box::new(e)),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for IRBexpr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => write!(f, "{lhs:?} {cmp_op} {rhs:?}",),
            IRBexpr::And(exprs) => write!(f, "AND {exprs:?}"),
            IRBexpr::Or(exprs) => write!(f, "OR {exprs:?}"),
            IRBexpr::Not(expr) => write!(f, "NOT {expr:?}"),
        }
    }
}

impl<T: Clone> Clone for IRBexpr<T> {
    fn clone(&self) -> Self {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => IRBexpr::Cmp(*cmp_op, lhs.clone(), rhs.clone()),
            IRBexpr::And(exprs) => IRBexpr::And(exprs.clone()),
            IRBexpr::Or(exprs) => IRBexpr::Or(exprs.clone()),
            IRBexpr::Not(expr) => IRBexpr::Not(expr.clone()),
        }
    }
}

impl<T: PartialEq> PartialEq for IRBexpr<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (IRBexpr::Cmp(op1, lhs1, rhs1), IRBexpr::Cmp(op2, lhs2, rhs2)) => {
                op1 == op2 && lhs1 == lhs2 && rhs1 == rhs2
            }
            (IRBexpr::And(lhs), IRBexpr::And(rhs)) => lhs == rhs,
            (IRBexpr::Or(lhs), IRBexpr::Or(rhs)) => lhs == rhs,
            (IRBexpr::Not(lhs), IRBexpr::Not(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

fn reduce_bool_expr<A, L>(
    exprs: impl IntoIterator<Item = IRBexpr<A>>,
    l: &L,
    cb: impl Fn(&L, &L::CellOutput, &L::CellOutput) -> Result<L::CellOutput>,
) -> Result<L::CellOutput>
where
    A: LowerableExpr<F = L::F>,
    L: ExprLowering + ?Sized,
{
    exprs
        .into_iter()
        .map(|e| e.lower(l))
        .reduce(|lhs, rhs| lhs.and_then(|lhs| rhs.and_then(|rhs| cb(l, &lhs, &rhs))))
        .ok_or_else(|| anyhow::anyhow!("Boolean expression with no elements"))
        .and_then(identity)
}

impl<F> IRBexpr<F> {}

impl<A: LowerableExpr> LowerableExpr for IRBexpr<A> {
    type F = A::F;

    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering<F = Self::F> + ?Sized,
    {
        match self {
            IRBexpr::Cmp(cmp_op, lhs, rhs) => {
                let lhs = lhs.lower(l)?;
                let rhs = rhs.lower(l)?;
                match cmp_op {
                    CmpOp::Eq => l.lower_eq(&lhs, &rhs),
                    CmpOp::Lt => l.lower_lt(&lhs, &rhs),
                    CmpOp::Le => l.lower_le(&lhs, &rhs),
                    CmpOp::Gt => l.lower_gt(&lhs, &rhs),
                    CmpOp::Ge => l.lower_ge(&lhs, &rhs),
                    CmpOp::Ne => l.lower_ne(&lhs, &rhs),
                }
            }
            IRBexpr::And(exprs) => reduce_bool_expr(exprs, l, L::lower_and),
            IRBexpr::Or(exprs) => reduce_bool_expr(exprs, l, L::lower_or),
            IRBexpr::Not(expr) => expr.lower(l).and_then(|e| l.lower_not(&e)),
        }
    }
}
