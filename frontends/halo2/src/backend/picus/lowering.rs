use super::expr::{self, PicusExpr};
use super::vars::{VarAllocator, VarKind};
use crate::backend::func::{ArgNo, FieldId, FuncIO};
use crate::backend::lowering::Lowering;
use crate::backend::resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver};
use crate::halo2::{
    AdviceQuery, Challenge, FixedQuery, InstanceQuery, PrimeField, Selector, Value,
};
use crate::value::steal;
use anyhow::{anyhow, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

pub type PicusModuleRef = Rc<RefCell<PicusModule>>;

struct PicusConstraint(PicusExpr);

#[derive(Default)]
pub struct PicusModule {
    constraints: Vec<PicusConstraint>,
    input_vars: HashMap<ArgNo, String>,
    output_vars: HashMap<FieldId, String>,
}

#[derive(Clone)]
pub struct PicusModuleLowering<F> {
    module: PicusModuleRef,
    _field: PhantomData<F>,
}

impl<'a, F> PicusModuleLowering<F> {
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

    fn allocate_input_var(&'a self, arg_no: ArgNo) -> &'a str {
        todo!()
    }

    fn allocate_output_var(&'a self, field_id: FieldId) -> &'a str {
        todo!()
    }

    fn allocate_temp_var(&'a self, temp: (usize, usize)) -> &'a str {
        todo!()
    }

    fn lower_resolved_query(&self, query: ResolvedQuery<F>) -> Result<Value<PicusExpr>> {
        todo!()
    }
}

impl<F: PrimeField> Lowering for PicusModuleLowering<F> {
    type CellOutput = PicusExpr;

    type F = F;

    fn generate_constraint(
        &self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<()> {
        let lhs = steal(lhs).ok_or_else(|| anyhow!("lhs value is unknown"))?;
        let rhs = steal(rhs).ok_or_else(|| anyhow!("rhs value is unknown"))?;
        self.module
            .borrow_mut()
            .constraints
            .push(PicusConstraint(expr::eq(&lhs, &rhs)));
        Ok(())
    }

    fn num_constraints(&self) -> usize {
        self.module.borrow().constraints.len()
    }

    fn generate_call(
        &self,
        _name: &str,
        _selectors: &[Value<Self::CellOutput>],
        _queries: &[Value<Self::CellOutput>],
    ) -> Result<()> {
        todo!()
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
        _expr: &Value<Self::CellOutput>,
        _scale: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        todo!()
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
            ResolvedSelector::Const(value) => self.lower_constant(&Self::F::from(value as u64)),
            ResolvedSelector::Arg(arg_no) => Ok(Value::known(expr::input_var(self, arg_no)?)),
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
        Ok(Value::known(expr::r#const(*f)))
    }
}

impl<'a, F> VarAllocator<'a> for PicusModuleLowering<F> {
    type Meta = FuncIO;

    fn allocate<M: Into<Self::Meta>>(&'a self, kind: &VarKind, meta: M) -> Result<&'a str> {
        let meta = meta.into();
        Ok(match kind {
            VarKind::Input => self.allocate_input_var(meta.try_into()?),
            VarKind::Output => self.allocate_output_var(meta.try_into()?),
            VarKind::Temporary => self.allocate_temp_var(meta.try_into()?),
        })
    }
}
