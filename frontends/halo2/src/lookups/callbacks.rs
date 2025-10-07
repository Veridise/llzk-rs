//! Structs for handling lookups from the client side.

use std::{borrow::Cow, cell::LazyCell};

use crate::{
    halo2::{Expression, Field},
    ir::stmt::IRStmt,
    temps::{ExprOrTemp, Temps},
};
use anyhow::{Error, Result};

use super::{Lookup, LookupTableRow};

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

/// Callback trait for defering to the client how to handle the logic of a lookup.
pub trait LookupCallbacks<F: Field> {
    /// Called on the list of lookups the circuit defines.
    ///
    /// By default loops over the lookups and calls [`LookupCallbacks::on_lookup`] on each.
    fn on_lookups<'syn>(
        &self,
        lookups: &[Lookup<'syn, F>],
        tables: &[&dyn LookupTableGenerator<F>],
        temps: &mut Temps,
    ) -> Result<IRStmt<ExprOrTemp<Cow<'syn, Expression<F>>>>> {
        lookups
            .iter()
            .zip(tables.iter())
            .map(|(lookup, table)| self.on_lookup(*lookup, *table, temps))
            .collect()
    }

    /// Called on each lookup the circuit defines.
    fn on_lookup<'syn>(
        &self,
        lookup: Lookup<'syn, F>,
        table: &dyn LookupTableGenerator<F>,
        temps: &mut Temps,
    ) -> Result<IRStmt<ExprOrTemp<Cow<'syn, Expression<F>>>>>;
}

pub(crate) struct DefaultLookupCallbacks;

impl<F: Field> LookupCallbacks<F> for DefaultLookupCallbacks {
    #[allow(unused_variables)]
    fn on_lookup<'syn>(
        &self,
        lookup: Lookup<'syn, F>,
        table: &dyn LookupTableGenerator<F>,
        temps: &mut Temps,
    ) -> Result<IRStmt<ExprOrTemp<Cow<'syn, Expression<F>>>>> {
        panic!("Target circuit has lookups but their behaviour was not specified");
    }
}
