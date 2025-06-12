use super::func::{ArgNo, FuncIO};
use crate::{
    gates::AnyQuery,
    halo2::{AdviceQuery, Field, FixedQuery, InstanceQuery, Selector, Value},
};
use anyhow::Result;

pub struct Bool(bool);

impl From<bool> for Bool {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl Bool {
    pub fn to_f<F>(self) -> F
    where
        F: Field,
    {
        if self.0 {
            F::ONE
        } else {
            F::ZERO
        }
    }
}

pub enum ResolvedSelector {
    // When the selector is used as argument.
    Const(Bool),
    // When the selector is used as formal.
    Arg(ArgNo),
}

impl From<ArgNo> for ResolvedSelector {
    fn from(value: ArgNo) -> Self {
        Self::Arg(value)
    }
}

impl From<bool> for ResolvedSelector {
    fn from(value: bool) -> Self {
        Self::Const(value.into())
    }
}

pub trait SelectorResolver {
    fn resolve_selector(&self, selector: &Selector) -> Result<ResolvedSelector>;
}

pub enum QueryKind {
    Advice,
    Fixed,
    Instance,
}

pub enum ResolvedQuery<F> {
    // Literal field value
    Lit(Value<F>),
    // An input or output of a function
    IO(FuncIO),
}

impl<F: Field> From<ArgNo> for ResolvedQuery<F> {
    fn from(value: ArgNo) -> Self {
        Self::IO(FuncIO::Arg(value))
    }
}

impl<F: Field> From<FuncIO> for ResolvedQuery<F> {
    fn from(value: FuncIO) -> Self {
        Self::IO(value)
    }
}

pub trait QueryResolver<F: Field> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>>;

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<ResolvedQuery<F>>;

    fn resolve_instance_query(&self, query: &InstanceQuery) -> Result<ResolvedQuery<F>>;

    fn resolve_any_query(&self, query: &AnyQuery) -> Result<ResolvedQuery<F>> {
        match query {
            AnyQuery::Advice(advice_query) => self.resolve_advice_query(advice_query),
            AnyQuery::Instance(instance_query) => self.resolve_instance_query(instance_query),
            AnyQuery::Fixed(fixed_query) => self.resolve_fixed_query(fixed_query),
        }
    }
}
