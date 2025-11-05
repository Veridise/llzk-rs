//! Types related to the tables read by lookups.

use anyhow::{Error, Result};
use ff::Field;
use std::{cell::LazyCell, ops::Index};

use crate::{expressions::ExpressionInfo, info_traits::QueryInfo, resolvers::Fixed};

/// Type alias for a result of creating a boxed slice representing the rows in the table.
pub type LookupTableBox<F> = Result<Box<[LookupTableRow<F>]>>;

/// Implementations of this trait compute the complete table for a lookup.
pub trait LookupTableGenerator<F> {
    /// Returns the lookup table.
    fn table(&self) -> Result<&[LookupTableRow<F>], &Error>;
}

/// Lazy lookup table generator.
pub(crate) struct LazyLookupTableGenerator<F, FN>
where
    FN: FnOnce() -> LookupTableBox<F>,
{
    table: LazyCell<LookupTableBox<F>, FN>,
}

impl<F, FN> LazyLookupTableGenerator<F, FN>
where
    FN: FnOnce() -> LookupTableBox<F>,
{
    /// Creates a new lazy generator using the given closure.
    pub fn new(f: FN) -> Self {
        Self {
            table: LazyCell::new(f),
        }
    }
}

impl<F: Field, FN: FnOnce() -> LookupTableBox<F>> LookupTableGenerator<F>
    for LazyLookupTableGenerator<F, FN>
{
    fn table(&self) -> Result<&[LookupTableRow<F>], &Error> {
        (*self.table).as_ref().map(AsRef::as_ref)
    }
}

impl<F, FN: FnOnce() -> LookupTableBox<F>> std::fmt::Debug for LazyLookupTableGenerator<F, FN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyLookupTableGenerator").finish()
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

impl<F, Q: QueryInfo<Kind = Fixed>> Index<Q> for LookupTableRow<F> {
    type Output = F;

    fn index(&self, index: Q) -> &Self::Output {
        &self[index.column_index()]
    }
}

//impl<F: std::fmt::Debug, E: ExpressionInfo + std::fmt::Debug> Index<E> for LookupTableRow<F> {
//    type Output = F;
//
//    fn index(&self, index: E) -> &Self::Output {
//        match index.as_fixed_query() {
//            Some(query) => &self[*query],
//            _ => panic!("Cannot index a lookup table row with expression {index:?}"),
//        }
//    }
//}
