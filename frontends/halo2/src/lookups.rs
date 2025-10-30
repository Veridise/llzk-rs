//! Structs for handling lookups.

use std::ops::Index;

use crate::{
    expressions::{EvaluableExpr, ExprBuilder, ExpressionInfo},
    gates::AnyQuery,
    halo2::{Expression, Field, FixedQuery},
    info_traits::ConstraintSystemInfo,
};
use anyhow::Result;

pub mod callbacks;

/// Defines a lookup as a list of pairs of expressions.
#[derive(Debug)]
pub struct Lookup<E> {
    name: String,
    idx: usize,
    inputs: Vec<E>,
    table: Vec<E>,
}

impl<E> std::fmt::Display for Lookup<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lookup {} '{}'", self.idx, self.name)
    }
}

/// Lightweight representation of a lookup that is cheap to copy
#[derive(Debug, Copy, Clone)]
pub struct LookupData<'syn, E> {
    /// Name of the lookup.
    pub name: &'syn str,
    /// Expressions representing the arguments of the lookup.
    pub arguments: &'syn [E],
    /// Expressions representing the columns of the table.
    pub table: &'syn [E],
}

fn query_from_table_expr<E: ExpressionInfo>(e: &E) -> Result<FixedQuery> {
    e.as_fixed_query()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("Table row expressions can only be fixed cell queries"))
}

fn compute_table_cells<'e, E: ExpressionInfo + 'e>(
    table: impl Iterator<Item = &'e E>,
) -> Result<Vec<AnyQuery>> {
    table
        .map(query_from_table_expr)
        .map(|e| e.map(Into::into))
        .collect()
}

impl<E> Lookup<E> {
    /// Returns the list of lookups defined in the constraint system.
    pub fn load<'syn, F: Field>(cs: &'syn dyn ConstraintSystemInfo<F, Polynomial = E>) -> Vec<Self>
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
    pub fn table_queries(&self) -> Result<Vec<AnyQuery>>
    where
        E: ExpressionInfo,
    {
        compute_table_cells(self.table.iter())
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

/// Represents a row in the lookup table that can be indexed by the columns participating in the
/// lookup.
#[derive(Debug)]
pub struct LookupTableRow<F> {
    // Maps the n-th index of the slice to the n-th column
    columns: Vec<usize>,
    table: Vec<F>,
}

impl<F: Copy> LookupTableRow<F> {
    pub(crate) fn new(columns: &[usize], table: Vec<F>) -> Self {
        Self {
            columns: columns.to_vec(),
            table,
        }
    }
}

impl<F> LookupTableRow<F> {
    fn col_to_index(&self, col: usize) -> Option<usize> {
        self.columns.iter().find(|c| **c == col).copied()
    }
}

impl<F> Index<usize> for LookupTableRow<F> {
    type Output = F;

    fn index(&self, index: usize) -> &Self::Output {
        let index = self.col_to_index(index).unwrap_or_else(|| {
            panic!(
                "Can't index with a column outside of the valid range {:?}",
                self.columns
            )
        });
        &self.table[index]
    }
}

impl<F> Index<FixedQuery> for LookupTableRow<F> {
    type Output = F;

    fn index(&self, index: FixedQuery) -> &Self::Output {
        &self[index.column_index()]
    }
}

impl<F: std::fmt::Debug> Index<Expression<F>> for LookupTableRow<F> {
    type Output = F;

    fn index(&self, index: Expression<F>) -> &Self::Output {
        match index {
            Expression::Fixed(query) => &self[query],
            _ => panic!("Cannot index a lookup table row with expression {index:?}"),
        }
    }
}
