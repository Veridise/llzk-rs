use super::{vars::VarKey, FeltWrap};
use crate::{
    backend::{
        func::FuncIO,
        lowering::Lowering,
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    halo2::{AdviceQuery, Challenge, FixedQuery, InstanceQuery, PrimeField, Selector, Value},
    ir::lift::LiftLowering,
    synthesis::regions::FQN,
    value::{steal, steal_many},
    LiftLike,
};
use anyhow::{anyhow, bail, Result};
use picus::{expr, stmt, ModuleLike as _};
use std::marker::PhantomData;

pub type PicusModuleRef = picus::ModuleRef<VarKey>;
type PicusExpr = picus::expr::Expr;

#[derive(Clone)]
pub struct PicusModuleLowering<F, L> {
    module: PicusModuleRef,
    lift_fixed: bool,
    _field: PhantomData<(F, L)>,
}

impl<F, L> PicusModuleLowering<F, L> {
    pub fn new(module: PicusModuleRef, lift_fixed: bool) -> Self {
        Self {
            module,
            lift_fixed,
            _field: Default::default(),
        }
    }
}

impl<'a, F: PrimeField, L: LiftLike<Inner = F>> PicusModuleLowering<F, L> {
    fn lower_binary_op<Fn, T: Clone>(
        &self,
        lhs: &Value<T>,
        rhs: &Value<T>,
        f: Fn,
    ) -> Result<Value<T>>
    where
        Fn: FnOnce(&T, &T) -> T,
    {
        Ok(lhs.clone().zip(rhs.clone()).map(|(lhs, rhs)| f(&lhs, &rhs)))
    }

    fn lower_unary_op<Fn, T: Clone>(&self, expr: &Value<T>, f: Fn) -> Result<Value<T>>
    where
        Fn: FnOnce(&T) -> T,
    {
        Ok(expr.clone().map(|e| f(&e)))
    }

    fn lower_resolved_query(
        &self,
        query: ResolvedQuery<L>,
        fqn: Option<FQN>,
    ) -> Result<Value<PicusExpr>> {
        Ok(match query {
            ResolvedQuery::Lit(value) => {
                let f = steal(&value).ok_or(anyhow!("Query resolved to an unknown value"));
                Value::known(self.lower(&f?, true)?)
            }
            ResolvedQuery::IO(func_io) => Value::known(expr::var(&self.module, (func_io, fqn))),
        })
    }
}

impl<F: PrimeField, L: LiftLike<Inner = F>> LiftLowering for PicusModuleLowering<F, L> {
    type F = F;

    type Output = PicusExpr;

    fn lower_constant(&self, f: &Self::F) -> Result<Self::Output> {
        Ok(expr::r#const(FeltWrap(*f)))
    }

    fn lower_lifted(&self, id: usize, f: Option<&Self::F>) -> Result<Self::Output> {
        if self.lift_fixed {
            Ok(expr::var(
                &self.module,
                VarKey::Lifted(FuncIO::Arg(0.into()), id),
            ))
        } else if let Some(f) = f {
            Ok(expr::r#const(FeltWrap(*f)))
        } else {
            bail!(
                "Lifted value did not have an inner value and the lowerer was not configured to lift"
            )
        }
    }

    fn lower_add(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output> {
        Ok(expr::add(lhs, rhs))
    }

    fn lower_sub(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output> {
        Ok(expr::sub(lhs, rhs))
    }

    fn lower_mul(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output> {
        Ok(expr::mul(lhs, rhs))
    }

    fn lower_neg(&self, expr: &Self::Output) -> Result<Self::Output> {
        Ok(expr::neg(expr))
    }

    fn lower_double(&self, expr: &Self::Output) -> Result<Self::Output> {
        Ok(expr::add(expr, expr))
    }

    fn lower_square(&self, expr: &Self::Output) -> Result<Self::Output> {
        Ok(expr::mul(expr, expr))
    }

    fn lower_invert(&self, _expr: &Self::Output) -> Result<Self::Output> {
        bail!("Inversion operation is not expressible in Picus")
    }

    fn lower_sqrt_ratio(&self, _lhs: &Self::Output, _rhs: &Self::Output) -> Result<Self::Output> {
        todo!()
    }

    fn lower_cond_select(
        &self,
        _cond: bool,
        _then: &Self::Output,
        _else: &Self::Output,
    ) -> Result<Self::Output> {
        bail!("Conditional select operation is not expressible in Picus")
    }
}

impl<F: PrimeField, L: LiftLike<Inner = F>> Lowering for PicusModuleLowering<F, L> {
    type CellOutput = PicusExpr;

    type F = L;

    fn generate_constraint(
        &self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<()> {
        let lhs = steal(lhs).ok_or_else(|| anyhow!("lhs value is unknown"))?;
        let rhs = steal(rhs).ok_or_else(|| anyhow!("rhs value is unknown"))?;
        self.module
            .borrow_mut()
            .add_constraint(expr::eq(&lhs, &rhs));
        Ok(())
    }

    fn num_constraints(&self) -> usize {
        self.module.constraints_len()
    }

    fn generate_call(
        &self,
        name: &str,
        selectors: &[Value<Self::CellOutput>],
        queries: &[Value<Self::CellOutput>],
    ) -> Result<()> {
        self.module.borrow_mut().add_stmt(stmt::call(
            name.to_owned(),
            steal_many(selectors)
                .ok_or_else(|| anyhow!("some selector value was unknown"))?
                .iter()
                .chain(
                    steal_many(queries)
                        .ok_or_else(|| anyhow!("some query value was unknown"))?
                        .iter(),
                )
                .map(Clone::clone)
                .collect(),
            0,
            &self.module,
        ));
        Ok(())
    }

    fn lower_sum<'a, 'l: 'a>(
        &'l self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        self.lower_binary_op(lhs, rhs, expr::add)
    }

    fn lower_product<'a>(
        &'a self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        self.lower_binary_op(lhs, rhs, expr::mul)
    }

    fn lower_neg<'a>(&'a self, expr: &Value<Self::CellOutput>) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        self.lower_unary_op(expr, expr::neg)
    }

    fn lower_scaled<'a>(
        &'a self,
        expr: &Value<Self::CellOutput>,
        scale: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        self.lower_binary_op(expr, scale, expr::mul)
    }

    fn lower_challenge<'a>(&'a self, _challenge: &Challenge) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        todo!()
    }

    fn lower_selector<'a, 'l: 'a>(
        &'l self,
        sel: &Selector,
        resolver: &dyn SelectorResolver,
    ) -> Result<Value<Self::CellOutput>>
    where
        PicusExpr: 'a,
    {
        match resolver.resolve_selector(sel)? {
            ResolvedSelector::Const(value) => Lowering::lower_constant(self, &value.to_f()),
            ResolvedSelector::Arg(arg_no) => Ok(Value::known(expr::var(&self.module, arg_no))),
        }
    }

    fn lower_advice_query<'a>(
        &'a self,
        query: &AdviceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        let (res, fqn) = resolver.resolve_advice_query(query)?;
        self.lower_resolved_query(res, fqn)
    }

    fn lower_instance_query<'a>(
        &'a self,
        query: &InstanceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        self.lower_resolved_query(resolver.resolve_instance_query(query)?, None)
    }

    fn lower_fixed_query<'a>(
        &'a self,
        query: &FixedQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        self.lower_resolved_query(resolver.resolve_fixed_query(query)?, None)
    }

    fn lower_constant<'a, 'f>(&'a self, f: &'f Self::F) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
        'a: 'f,
    {
        Ok(Value::known(self.lower(f, true)?))
    }
}
