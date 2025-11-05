use std::borrow::Cow;

use crate::{
    backend::func::{ArgNo, FuncIO},
    halo2::{Field, Value},
    info_traits::{QueryInfo, SelectorInfo},
};
use anyhow::Result;

mod sealed {
    /// Sealed trait pattern to avoid clients implementing the trait [`super::QueryKind`] on
    /// external types.
    pub trait QK {}
}

/// Marker trait for defining the kind of a query.
pub trait QueryKind: sealed::QK {}

/// Marker for fixed cell queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fixed;

impl sealed::QK for Fixed {}
impl QueryKind for Fixed {}

/// Marker for advice cell queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Advice;

impl sealed::QK for Advice {}
impl QueryKind for Advice {}

/// Marker for instance cell queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Instance;

impl sealed::QK for Instance {}
impl QueryKind for Instance {}

pub trait ResolversProvider<F> {
    fn query_resolver(&self) -> &dyn QueryResolver<F>;
    fn selector_resolver(&self) -> &dyn SelectorResolver;
}

pub(crate) fn boxed_resolver<'a, F: Field, T: ResolversProvider<F> + 'a>(
    t: T,
) -> Box<dyn ResolversProvider<F> + 'a> {
    Box::new(t)
}

impl<Q, F, S> ResolversProvider<F> for (Q, S)
where
    Q: QueryResolver<F> + Clone,
    F: Field,
    S: SelectorResolver + Clone,
{
    fn query_resolver(&self) -> &dyn QueryResolver<F> {
        &self.0
    }

    fn selector_resolver(&self) -> &dyn SelectorResolver {
        &self.1
    }
}

impl<T, F> ResolversProvider<F> for T
where
    T: QueryResolver<F> + SelectorResolver + Clone,
    F: Field,
{
    fn query_resolver(&self) -> &dyn QueryResolver<F> {
        self
    }

    fn selector_resolver(&self) -> &dyn SelectorResolver {
        self
    }
}

/// Represents the value of selector.
#[derive(Debug)]
pub struct Bool(bool);

impl From<bool> for Bool {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl Bool {
    pub fn to_f<F>(&self) -> F
    where
        F: Field,
    {
        if self.0 { F::ONE } else { F::ZERO }
    }
}

/// Possible values when resolving a selector.
#[derive(Debug)]
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

/// Resolver that returns the value or the variable that is representing the selector.
pub trait SelectorResolver {
    /// Resolved the selector and returns its value.
    fn resolve_selector(&self, selector: &dyn SelectorInfo) -> Result<ResolvedSelector>;
}

/// Possible results of resolving a query.
#[derive(Copy, Clone, Debug)]
pub enum ResolvedQuery<F> {
    // Literal field value
    Lit(F),
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

/// Resolver trait that only supports fixed cell queries.
pub trait FixedQueryResolver<F: Field> {
    /// Resolved the fixed query and returns its assigned value during synthesis.
    fn resolve_query(&self, query: &dyn QueryInfo<Kind = Fixed>, row: usize) -> Result<F>;
}

/// Resolver trait that converts a query to a cell into a constant value or a variable.
pub trait QueryResolver<F: Field> {
    /// Resolves a fixed query.
    fn resolve_fixed_query(&self, query: &dyn QueryInfo<Kind = Fixed>) -> Result<ResolvedQuery<F>>;

    /// Resolves an advice query.
    fn resolve_advice_query(
        &self,
        query: &dyn QueryInfo<Kind = Advice>,
    ) -> Result<ResolvedQuery<F>>;

    /// Resolves an instance query.
    fn resolve_instance_query(
        &self,
        query: &dyn QueryInfo<Kind = Instance>,
    ) -> Result<ResolvedQuery<F>>;
}

impl<F: Field, Q: QueryResolver<F> + Clone> QueryResolver<F> for Cow<'_, Q> {
    fn resolve_fixed_query(&self, query: &dyn QueryInfo<Kind = Fixed>) -> Result<ResolvedQuery<F>> {
        self.as_ref().resolve_fixed_query(query)
    }

    fn resolve_advice_query(
        &self,
        query: &dyn QueryInfo<Kind = Advice>,
    ) -> Result<ResolvedQuery<F>> {
        self.as_ref().resolve_advice_query(query)
    }

    fn resolve_instance_query(
        &self,
        query: &dyn QueryInfo<Kind = Instance>,
    ) -> Result<ResolvedQuery<F>> {
        self.as_ref().resolve_instance_query(query)
    }
}

impl<S: SelectorResolver + Clone> SelectorResolver for Cow<'_, S> {
    fn resolve_selector(&self, selector: &dyn SelectorInfo) -> Result<ResolvedSelector> {
        self.as_ref().resolve_selector(selector)
    }
}
