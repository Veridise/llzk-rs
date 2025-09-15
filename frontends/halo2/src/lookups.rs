//! Structs for handling lookups.

use std::ops::Index;

use crate::{
    backend::codegen::lookup::query_from_table_expr,
    gates::AnyQuery,
    halo2::{ConstraintSystem, Expression, Field, FixedQuery},
};
use anyhow::Result;

pub mod callbacks;

/// Lightweight representation of a lookup that is cheap to copy
#[derive(Clone, Copy, Debug)]
pub struct Lookup<'a, F: Field> {
    name: &'a str,
    idx: usize,
    inputs: &'a [Expression<F>],
    table: &'a [Expression<F>],
}

impl<F: Field> std::fmt::Display for Lookup<'_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lookup {} '{}'", self.idx, self.name)
    }
}

/// A heavier representation of the lookup
#[derive(Clone, Debug)]
pub struct LookupData {
    table_queries: Vec<AnyQuery>,
}

impl<'a, F: Field> TryFrom<Lookup<'a, F>> for LookupData {
    type Error = anyhow::Error;

    fn try_from(lookup: Lookup<'a, F>) -> std::result::Result<Self, Self::Error> {
        let table_queries = compute_table_cells(lookup.table.iter())?;
        Ok(LookupData { table_queries })
    }
}

fn compute_table_cells<'a, F: Field>(
    table: impl Iterator<Item = &'a Expression<F>>,
) -> Result<Vec<AnyQuery>> {
    table
        .map(query_from_table_expr)
        .map(|e| e.map(Into::into))
        .collect()
}

impl<'a, F: Field> Lookup<'a, F> {
    /// Returns the list of lookups defined in the constraint system.
    pub fn load(cs: &'a ConstraintSystem<F>) -> Vec<Self> {
        cs.lookups()
            .iter()
            .enumerate()
            .map(|(idx, a)| Self::new(idx, a.name(), a.input_expressions(), a.table_expressions()))
            .collect()
    }

    fn new(
        idx: usize,
        name: &'a str,
        inputs: &'a [Expression<F>],
        table: &'a [Expression<F>],
    ) -> Self {
        Self {
            idx,
            name,
            inputs,
            table,
        }
    }

    /// Name given to the lookup.
    pub fn name(&self) -> &str {
        self.name
    }

    /// Returns the index of the lookup.
    pub fn idx(&self) -> usize {
        self.idx
    }

    /// Returns the list of expressions used to query the lookup table.
    pub fn expressions(&self) -> impl Iterator<Item = (&'a Expression<F>, &'a Expression<F>)> + 'a {
        self.inputs.iter().zip(self.table)
    }

    /// Returns the inputs of the queries.
    pub fn inputs(&self) -> &'a [Expression<F>] {
        self.inputs
    }

    /// Returns the queries to the lookup table.
    pub fn table_queries(&self) -> Result<Vec<AnyQuery>> {
        compute_table_cells(self.table.iter())
    }

    /// Returns an expression for the query to the n-th column in the table.
    pub fn expr_for_column(&self, col: usize) -> Result<&Expression<F>> {
        self.table_queries()?
            .into_iter()
            .enumerate()
            .find(|(_, q)| q.column_index() == col)
            .ok_or_else(|| anyhow::anyhow!("Column {col} not found"))
            .map(|(idx, _)| &self.inputs[idx])
    }
}

impl LookupData {
    /// Returns the list of queries.
    pub fn output_queries(&self) -> &[AnyQuery] {
        &self.table_queries
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
