use std::{
    collections::HashMap,
    convert::identity,
    hash::{DefaultHasher, Hash as _, Hasher as _},
};

use crate::{
    backend::{
        codegen::lookup::{contains_fixed, query_from_table_expr},
        func::FuncIO,
    },
    gates::{compute_gate_arity, AnyQuery},
    halo2::{Column, Expression, Field, Selector},
    ir::{expr::IRExpr, stmt::IRStmt, IRModule},
    synthesis::{regions::RegionRowLike, CircuitSynthesis},
};
use anyhow::{bail, Result};

use super::{Lookup, LookupKind, LookupTableRow};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LookupIO {
    I,
    O,
}

pub trait LookupCallbacks<F: Field> {
    /// This callback offers the possibility of creating a module that encapsulates the lookup.
    /// By default returns Ok(None) indicating that no module is created.
    fn on_body(
        &self,
        _kind: &LookupKind,
        _io: &dyn Iterator<Item = (usize, LookupIO)>,
    ) -> Result<Option<IRModule<IRExpr<F>>>> {
        Ok(None)
    }

    fn on_lookup(
        &self,
        region_row: &dyn RegionRowLike,
        lookup: Lookup<F>,
        table: &[LookupTableRow<F>],
    ) -> Result<Vec<IRStmt<IRExpr<F>>>>;

    /// This callbacks ask for the kind of io a column is. By default returns None.
    fn assign_io_kind(&self, expr: &Expression<F>, column: usize) -> Option<LookupIO> {
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
    ) -> Result<Option<IRModule<IRExpr<F>>>> {
        lookups_panic()
    }

    fn on_lookup(
        &self,
        _region_row: &dyn RegionRowLike,
        _lookup: Lookup<F>,
        _table: &[LookupTableRow<F>],
    ) -> Result<Vec<IRStmt<IRExpr<F>>>> {
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
    fn on_lookup(
        &self,
        _region_row: &dyn RegionRowLike,
        _lookup: Lookup<F>,
        _table: &[LookupTableRow<F>],
    ) -> Result<Vec<IRStmt<IRExpr<F>>>> {
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
