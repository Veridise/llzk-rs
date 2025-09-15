use std::{borrow::Cow, cell::LazyCell};

use crate::{
    halo2::{Expression, Field},
    ir::stmt::IRStmt,
};
use anyhow::{Error, Result};

use super::{Lookup, LookupTableRow};

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
    fn on_lookup<'a>(
        &self,
        lookup: Lookup<'a, F>,
        table: &dyn LookupTableGenerator<F>,
    ) -> Result<IRStmt<Cow<'a, Expression<F>>>>;
}

pub(crate) struct DefaultLookupCallbacks;

impl<F: Field> LookupCallbacks<F> for DefaultLookupCallbacks {
    #[allow(unused_variables)]
    fn on_lookup<'a>(
        &self,
        lookup: Lookup<'a, F>,
        table: &dyn LookupTableGenerator<F>,
    ) -> Result<IRStmt<Cow<'a, Expression<F>>>> {
        panic!("Target circuit has lookups but their behaviour was not specified");
    }
}
