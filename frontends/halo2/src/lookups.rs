//! Structs for handling lookups.

use anyhow::Result;
use ff::Field;
use halo2_frontend_core::{
    expressions::{EvaluableExpr, ExprBuilder, ExpressionInfo},
    info_traits::{ConstraintSystemInfo, QueryInfo as _},
};

pub mod callbacks;
pub mod table;

/// Defines a lookup as a list of pairs of expressions.
#[derive(Debug)]
pub struct Lookup<E> {
    name: String,
    idx: usize,
    inputs: Vec<E>,
    table: Vec<E>,
}

impl<E> Lookup<E> {
    /// Returns the list of lookups defined in the constraint system.
    pub fn load<F: Field>(cs: &dyn ConstraintSystemInfo<F, Polynomial = E>) -> Vec<Self>
    where
        E: EvaluableExpr<F> + Clone + ExpressionInfo + ExprBuilder<F>,
    {
        cs.lookups()
            .iter()
            .enumerate()
            .map(|(idx, a)| Self::new(idx, a.name, a.arguments, a.table))
            .collect()
    }

    fn new(idx: usize, name: &str, inputs: &[E], table: &[E]) -> Self
    where
        E: Clone,
    {
        Self {
            idx,
            name: name.to_string(),
            inputs: inputs.to_vec(),
            table: table.to_vec(),
        }
    }

    /// Name given to the lookup.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the index of the lookup.
    pub fn idx(&self) -> usize {
        self.idx
    }

    /// Returns the list of expressions used to query the lookup table.
    pub fn expressions(&self) -> impl Iterator<Item = (&E, &E)> {
        self.inputs.iter().zip(self.table.iter())
    }

    /// Returns the inputs of the queries.
    pub fn inputs(&self) -> &[E] {
        &self.inputs
    }

    /// Returns the queries to the lookup table.
    pub fn table_queries(&self) -> Result<Vec<E::FixedQuery>>
    where
        E: ExpressionInfo,
    {
        self.table
            .iter()
            .map(|e| {
                e.as_fixed_query().copied().ok_or_else(|| {
                    anyhow::anyhow!("Table row expressions can only be fixed cell queries")
                })
            })
            .collect()
    }

    /// Returns an expression for the query to the n-th column in the table.
    pub fn expr_for_column(&self, col: usize) -> Result<&E>
    where
        E: ExpressionInfo,
    {
        self.table_queries()?
            .into_iter()
            .enumerate()
            .find(|(_, q)| q.column_index() == col)
            .ok_or_else(|| anyhow::anyhow!("Column {col} not found"))
            .map(|(idx, _)| &self.inputs[idx])
    }
}

impl<E> std::fmt::Display for Lookup<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lookup {} '{}'", self.idx, self.name)
    }
}
