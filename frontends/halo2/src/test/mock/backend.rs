use crate::{
    backend::{
        func::FuncIO,
        resolvers::{QueryResolver, ResolvedQuery},
    },
    halo2::{Fr, Value},
    value::{steal, steal_many},
};
use std::{
    cell::{Ref, RefCell},
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Deref,
    rc::Rc,
};

use crate::{
    backend::{
        func::{ArgNo, FieldId},
        lowering::Lowering,
        resolvers::ResolvedSelector,
        Backend,
    },
    gates::AnyQuery,
    halo2::{
        Advice, AdviceQuery, Challenge, Expression, Field, FixedQuery, Instance, InstanceQuery,
        Selector,
    },
    ir::CircuitStmt,
    CircuitIO,
};
use anyhow::{bail, Result};

type SharedFuncRef = Rc<RefCell<MockFunc>>;

#[derive(Default)]
pub struct MockContext {
    gates: Vec<SharedFuncRef>,
    main: Option<SharedFuncRef>,
    gate_names: HashSet<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MockOutput {
    pub gates: Vec<MockFunc>,
    pub main: Option<MockFunc>,
}

pub struct MockBackend(RefCell<MockContext>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MockExprIR {
    Arg(ArgNo),
    Field(FieldId),
    Sum(usize, usize),
    Product(usize, usize),
    Neg(usize),
    Scaled(usize, usize),
    Const(Fr),
    Temp(usize, usize),
    Constraint(usize, usize),
    Call(String, Vec<usize>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MockFunc {
    pub name: String,
    pub args: Vec<ArgNo>,
    pub fields: Vec<FieldId>,
    pub exprs: Vec<MockExprIR>,
}

impl MockFunc {
    fn shared(name: &str, arg_count: usize, field_count: Option<usize>) -> SharedFuncRef {
        Rc::new(Self::new(name, arg_count, field_count).into())
    }

    fn new(name: &str, arg_count: usize, field_count: Option<usize>) -> Self {
        let args = (0..arg_count).map(Into::into).collect();
        let fields = field_count
            .map(|field_count| (0..field_count).map(Into::into).collect())
            .unwrap_or(Default::default());

        Self {
            name: name.to_owned(),
            args,
            fields,
            exprs: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct MockFuncRef(SharedFuncRef);

impl MockFuncRef {
    fn constraints_len(&self) -> usize {
        self.0
            .borrow()
            .exprs
            .iter()
            .filter(|e| match e {
                MockExprIR::Constraint(_, _) => true,
                _ => false,
            })
            .count()
    }

    fn add_constraint(&self, lhs: Value<usize>, rhs: Value<usize>) {
        let lhs = steal(&lhs).unwrap();
        let rhs = steal(&rhs).unwrap();

        self.0
            .borrow_mut()
            .exprs
            .push(MockExprIR::Constraint(lhs, rhs));
    }

    fn add_call(&self, name: String, selectors: &[Value<usize>], queries: &[Value<usize>]) {
        self.0.borrow_mut().exprs.push(MockExprIR::Call(
            name,
            steal_many(selectors)
                .unwrap()
                .iter()
                .chain(steal_many(queries).unwrap().iter())
                .map(Clone::clone)
                .collect(),
        ));
    }

    fn push_expr(&self, expr: MockExprIR) -> usize {
        let idx = self.0.borrow().exprs.len();
        self.0.borrow_mut().exprs.push(expr);
        idx
    }
}

impl Lowering for MockFuncRef {
    type CellOutput = usize;
    type F = Fr;

    fn generate_constraint(
        &self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<()> {
        self.add_constraint(*lhs, *rhs);
        Ok(())
    }

    fn num_constraints(&self) -> usize {
        self.constraints_len()
    }

    fn generate_call(
        &self,
        name: &str,
        selectors: &[Value<Self::CellOutput>],
        queries: &[Value<Self::CellOutput>],
    ) -> Result<()> {
        self.add_call(name.to_owned(), selectors, queries);
        Ok(())
    }

    fn lower_sum(
        &self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>> {
        Ok(lhs
            .zip(*rhs)
            .map(|(lhs, rhs)| self.push_expr(MockExprIR::Sum(lhs, rhs))))
    }

    fn lower_product(
        &self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>> {
        Ok(lhs
            .zip(*rhs)
            .map(|(lhs, rhs)| self.push_expr(MockExprIR::Product(lhs, rhs))))
    }

    fn lower_neg(&self, expr: &Value<Self::CellOutput>) -> Result<Value<Self::CellOutput>> {
        Ok(expr.map(|expr| self.push_expr(MockExprIR::Neg(expr))))
    }

    fn lower_scaled(
        &self,
        expr: &Value<Self::CellOutput>,
        scale: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>> {
        Ok(expr
            .zip(*scale)
            .map(|(expr, scale)| self.push_expr(MockExprIR::Scaled(expr, scale))))
    }

    fn lower_challenge(&self, _challenge: &Challenge) -> Result<Value<Self::CellOutput>> {
        todo!()
    }

    fn lower_selector(
        &self,
        sel: &Selector,
        resolver: &dyn crate::backend::resolvers::SelectorResolver,
    ) -> Result<Value<Self::CellOutput>> {
        let resolved = resolver.resolve_selector(sel)?;
        Ok(Value::known(match resolved {
            ResolvedSelector::Const(value) => {
                self.push_expr(MockExprIR::Const(if value { Fr::ONE } else { Fr::ZERO }))
            }
            ResolvedSelector::Arg(arg_no) => self.push_expr(MockExprIR::Arg(arg_no)),
        }))
    }

    fn lower_advice_query(
        &self,
        query: &AdviceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>> {
        let resolved = resolver.resolve_advice_query(query)?;

        Ok(match resolved {
            ResolvedQuery::Lit(_) => unreachable!(),
            ResolvedQuery::IO(func_io) => Value::known(self.push_expr(match func_io {
                FuncIO::Arg(arg_no) => MockExprIR::Arg(arg_no),
                FuncIO::Field(field_id) => MockExprIR::Field(field_id),
                FuncIO::Temp(col, row) => MockExprIR::Temp(col, row),
            })),
        })
    }

    fn lower_instance_query(
        &self,
        query: &InstanceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>> {
        let resolved = resolver.resolve_instance_query(query)?;

        Ok(match resolved {
            ResolvedQuery::Lit(_) => unreachable!(),
            ResolvedQuery::IO(func_io) => Value::known(self.push_expr(match func_io {
                FuncIO::Arg(arg_no) => MockExprIR::Arg(arg_no),
                FuncIO::Field(field_id) => MockExprIR::Field(field_id),
                FuncIO::Temp(col, row) => MockExprIR::Temp(col, row),
            })),
        })
    }

    fn lower_fixed_query(
        &self,
        query: &FixedQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>> {
        let resolved = resolver.resolve_fixed_query(query)?;

        Ok(match resolved {
            ResolvedQuery::Lit(value) => value.map(|f| self.push_expr(MockExprIR::Const(f))),
            ResolvedQuery::IO(func_io) => Value::known(self.push_expr(match func_io {
                FuncIO::Arg(arg_no) => MockExprIR::Arg(arg_no),
                FuncIO::Field(field_id) => MockExprIR::Field(field_id),
                FuncIO::Temp(col, row) => MockExprIR::Temp(col, row),
            })),
        })
    }

    fn lower_constant(&self, f: &Self::F) -> Result<Value<Self::CellOutput>> {
        Ok(Value::known(self.push_expr(MockExprIR::Const(*f))))
    }
}

impl<'c> Backend<'c, (), MockOutput> for MockBackend {
    type FuncOutput = MockFuncRef;
    type F = Fr;

    fn initialize(_: ()) -> Self {
        Self(Default::default())
    }

    fn generate_output<'o>(&'c self) -> Result<MockOutput>
    where
        MockOutput: 'o,
    {
        let clone_func = |func: &SharedFuncRef| func.borrow().clone();
        let ctx = self.0.borrow();
        let gates = ctx.gates.iter().map(clone_func).collect();
        let main = ctx.main.as_ref().map(clone_func);
        Ok(MockOutput { gates, main })
    }

    fn define_gate_function<'f>(
        &'c self,
        name: &str,
        selectors: &[&Selector],
        queries: &[AnyQuery],
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        let mut ctx = self.0.borrow_mut();
        if ctx.gate_names.contains(name) {
            bail!("Gate function for '{name}' defined twice!");
        }
        let func = MockFunc::shared(name, selectors.len() + queries.len(), None);

        ctx.gate_names.insert(name.to_owned());
        ctx.gates.push(func.clone());
        Ok(MockFuncRef(func))
    }

    fn define_main_function<'f>(
        &'c self,
        advice_io: &CircuitIO<Advice>,
        instance_io: &CircuitIO<Instance>,
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        if self.0.borrow().main.is_some() {
            bail!("Main function defined twice!");
        }
        let arg_count = instance_io.inputs().len() + advice_io.inputs().len();
        let field_count = instance_io.outputs().len() + advice_io.outputs().len();

        let func = MockFunc::shared("Main", arg_count, Some(field_count));
        self.0.borrow_mut().main.replace(func.clone());
        Ok(MockFuncRef(func))
    }
}
