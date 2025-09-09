use std::{borrow::Cow, cell::LazyCell};

use crate::{
    backend::codegen::lookup::contains_fixed,
    halo2::{Expression, Field},
    ir::{stmt::IRStmt, IRModule},
};
use anyhow::{Error, Result};

use super::{Lookup, LookupKind, LookupTableRow};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LookupIO {
    I,
    O,
}

pub type LookupTableBox<F> = Result<Box<[LookupTableRow<F>]>>;

pub trait LookupTableGenerator<F> {
    fn table(&self) -> Result<&[LookupTableRow<F>], &Error>;
}

pub struct LazyLookupTableGenerator<F, FN>
where
    FN: FnOnce() -> LookupTableBox<F>,
{
    table: LazyCell<LookupTableBox<F>, FN>,
}

impl<F, FN> LazyLookupTableGenerator<F, FN>
where
    FN: FnOnce() -> LookupTableBox<F>,
{
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

pub trait LookupCallbacks<F: Field> {
    /// This callback offers the possibility of creating a module that encapsulates the lookup.
    /// By default returns Ok(None) indicating that no module is created.
    fn on_body(
        &self,
        _kind: &LookupKind,
        _io: &dyn Iterator<Item = (usize, LookupIO)>,
    ) -> Result<Option<IRModule<Expression<F>>>> {
        Ok(None)
    }

    fn on_lookup<'a>(
        &self,
        lookup: Lookup<'a, F>,
        table: &dyn LookupTableGenerator<F>,
    ) -> Result<IRStmt<Cow<'a, Expression<F>>>>;

    /// This callbacks ask for the kind of io a column is. By default returns None.
    fn assign_io_kind(&self, _expr: &Expression<F>, _column: usize) -> Option<LookupIO> {
        None
    }
}

pub(crate) struct DefaultLookupCallbacks;

fn lookups_panic() -> ! {
    panic!("Target circuit has lookups but their behaviour was not specified");
}

impl<F: Field> LookupCallbacks<F> for DefaultLookupCallbacks {
    fn on_body(
        &self,
        _kind: &LookupKind,
        _io: &dyn Iterator<Item = (usize, LookupIO)>,
    ) -> Result<Option<IRModule<Expression<F>>>> {
        lookups_panic()
    }

    fn on_lookup<'a>(
        &self,
        _lookup: Lookup<'a, F>,
        _table: &dyn LookupTableGenerator<F>,
    ) -> Result<IRStmt<Cow<'a, Expression<F>>>> {
        lookups_panic()
    }

    fn assign_io_kind(&self, _expr: &Expression<F>, _column: usize) -> Option<LookupIO> {
        lookups_panic()
    }
}

/// Implements a callback that assigns expressions with fixed columns as inputs and the rest as
/// outputs. It's meant to serve as a foundation for custom lookup callbacks.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FixedTagLookup;

impl<F: Field> LookupCallbacks<F> for FixedTagLookup {
    fn on_lookup<'a>(
        &self,
        _lookup: Lookup<'a, F>,
        _table: &dyn LookupTableGenerator<F>,
    ) -> Result<IRStmt<Cow<'a, Expression<F>>>> {
        unreachable!()
    }

    fn assign_io_kind(&self, expr: &Expression<F>, _column: usize) -> Option<LookupIO> {
        if contains_fixed(&expr) {
            Some(LookupIO::I)
        } else {
            Some(LookupIO::O)
        }
    }
}
