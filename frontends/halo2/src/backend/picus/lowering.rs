use super::expr::{self, PicusExpr};
use super::output::PicusModule;
use super::stmt;
use super::vars::{VarAllocator, VarStr};
use crate::backend::func::FuncIO;
use crate::backend::lowering::Lowering;
use crate::backend::resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver};
use crate::halo2::{
    AdviceQuery, Challenge, FixedQuery, InstanceQuery, PrimeField, Selector, Value,
};
use crate::ir::lift::LiftLowering;
use crate::value::{steal, steal_many};
use crate::Lift;
use anyhow::{anyhow, bail, Result};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

pub type PicusModuleRef = Rc<RefCell<PicusModule>>;

#[derive(Clone)]
pub struct PicusModuleLowering<F> {
    module: PicusModuleRef,
    _field: PhantomData<F>,
}

impl<F> From<PicusModuleRef> for PicusModuleLowering<F> {
    fn from(module: PicusModuleRef) -> Self {
        Self {
            module,
            _field: Default::default(),
        }
    }
}

impl<'a, F: PrimeField> PicusModuleLowering<F> {
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

    fn lower_resolved_query(&self, query: ResolvedQuery<Lift<F>>) -> Result<Value<PicusExpr>> {
        Ok(match query {
            ResolvedQuery::Lit(value) => {
                let f = steal(&value).ok_or(anyhow!("Query resolved to an unknown value"));
                Value::known(self.lower(&f?, true)?)
            }
            ResolvedQuery::IO(func_io) => Value::known(expr::var(self, func_io)),
        })
    }
}

impl<F: PrimeField> LiftLowering for PicusModuleLowering<F> {
    type F = F;

    type Output = PicusExpr;

    fn lower_constant(&self, f: &Self::F) -> Result<Self::Output> {
        Ok(expr::r#const(*f))
    }

    fn lower_lifted(&self, id: usize) -> Result<Self::Output> {
        Ok(expr::lifted_input(self, id))
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

impl<F: PrimeField> Lowering for PicusModuleLowering<F> {
    type CellOutput = PicusExpr;

    type F = Lift<F>;

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
        self.module.borrow().constraints_len()
    }

    fn generate_call(
        &self,
        name: &str,
        selectors: &[Value<Self::CellOutput>],
        queries: &[Value<Self::CellOutput>],
    ) -> Result<()> {
        self.module.borrow_mut().add_call(stmt::call(
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
            self,
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
            ResolvedSelector::Arg(arg_no) => Ok(Value::known(expr::var(self, arg_no))),
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
        self.lower_resolved_query(resolver.resolve_advice_query(query)?)
    }

    fn lower_instance_query<'a>(
        &'a self,
        query: &InstanceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        self.lower_resolved_query(resolver.resolve_instance_query(query)?)
    }

    fn lower_fixed_query<'a>(
        &'a self,
        query: &FixedQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        self.lower_resolved_query(resolver.resolve_fixed_query(query)?)
    }

    fn lower_constant<'a, 'f>(&'a self, f: &'f Self::F) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
        'a: 'f,
    {
        Ok(Value::known(self.lower(&f, true)?))
    }
}

impl<F> VarAllocator for PicusModuleLowering<F> {
    type Kind = FuncIO;

    fn allocate<K: Into<Self::Kind>>(&self, kind: K) -> VarStr {
        let mut module = self.module.borrow_mut();
        module.add_var(Some(kind.into()))
    }

    fn allocate_temp(&self) -> VarStr {
        let mut module = self.module.borrow_mut();
        module.add_var(None)
    }

    fn allocate_lifted_input(&self, id: usize) -> VarStr {
        let mut module = self.module.borrow_mut();
        module.add_lifted_input(id)
    }
}
