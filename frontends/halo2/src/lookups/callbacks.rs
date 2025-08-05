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
    ir::expr::IRExpr,
    synthesis::{regions::RegionRowLike, CircuitSynthesis},
    CircuitStmt,
};
use anyhow::{bail, Result};

use super::{Lookup, LookupKind};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LookupIO {
    I,
    O,
}

pub trait LookupCallbacks<F: Field> {
    fn on_body(
        &self,
        kind: &LookupKind,
        inputs: &[FuncIO],
        outputs: &[FuncIO],
    ) -> Result<Vec<CircuitStmt<IRExpr<F>>>>;

    fn on_call(
        &self,
        region_row: &dyn RegionRowLike,
        lookup: Lookup<F>,
    ) -> Result<Vec<CircuitStmt<IRExpr<F>>>>;

    fn assign_io_kind(&self, expr: &Expression<F>, column: usize) -> LookupIO;
}

pub(crate) struct DefaultLookupCallbacks;

fn lookups_panic() -> ! {
    panic!("Target circuit has lookups but their behaviour was not specified");
}

impl<F: Field> LookupCallbacks<F> for DefaultLookupCallbacks {
    fn on_body(
        &self,
        _kind: &LookupKind,
        _inputs: &[FuncIO],
        _outputs: &[FuncIO],
    ) -> Result<Vec<CircuitStmt<IRExpr<F>>>> {
        lookups_panic()
    }

    fn on_call(
        &self,
        _region_row: &dyn RegionRowLike,
        _lookup: Lookup<F>,
    ) -> Result<Vec<CircuitStmt<IRExpr<F>>>> {
        lookups_panic()
    }

    fn assign_io_kind(&self, _expr: &Expression<F>, _column: usize) -> LookupIO {
        lookups_panic()
    }
}

/// Implements a callback that assigns expressions with fixed columns as inputs and the rest as
/// outputs. It's meant to serve as a foundation for custom lookup callbacks.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FixedTagLookup;

impl<F: Field> LookupCallbacks<F> for FixedTagLookup {
    fn on_body(
        &self,
        _kind: &LookupKind,
        _inputs: &[FuncIO],
        _outputs: &[FuncIO],
    ) -> Result<Vec<CircuitStmt<IRExpr<F>>>> {
        Ok(vec![])
    }

    fn on_call(
        &self,
        _region_row: &dyn RegionRowLike,
        _lookup: Lookup<F>,
    ) -> Result<Vec<CircuitStmt<IRExpr<F>>>> {
        Ok(vec![])
    }

    fn assign_io_kind(&self, expr: &Expression<F>, column: usize) -> LookupIO {
        if contains_fixed(&expr) {
            LookupIO::I
        } else {
            LookupIO::O
        }
    }
}
