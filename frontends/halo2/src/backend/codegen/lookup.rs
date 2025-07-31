use std::{
    convert::identity,
    hash::{DefaultHasher, Hash as _, Hasher as _},
    ops::BitOr,
};

use crate::{
    gates::{compute_gate_arity, AnyQuery},
    halo2::{Expression, Field, FixedQuery, Selector},
    synthesis::CircuitSynthesis,
};
use anyhow::{anyhow, Result};

use super::strats::GateScopedResolver;

pub mod codegen;

#[derive(Clone)]
pub struct Lookup<'a, F: Field> {
    name: &'a str,
    idx: usize,
    kind: LookupKind,
    inputs: &'a [Expression<F>],
    table: &'a [Expression<F>],
    selectors: Vec<&'a Selector>,
    queries: Vec<AnyQuery>,
    table_queries: Vec<AnyQuery>,
}

pub fn query_from_table_expr<F: Field>(e: &Expression<F>) -> Result<FixedQuery> {
    match e {
        Expression::Fixed(fixed_query) => Ok(*fixed_query),
        _ => Err(anyhow!(
            "Table row expressions can only be fixed cell queries"
        )),
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
    pub fn load(syn: &'a CircuitSynthesis<F>) -> Result<Vec<Self>> {
        syn.cs()
            .lookups()
            .iter()
            .enumerate()
            .map(|(idx, a)| {
                //let inputs = a.input_expressions();
                Self::new(
                    idx,
                    a.name(),
                    &a.input_expressions(),
                    &a.table_expressions(),
                )
            })
            .collect()
    }

    fn new(
        idx: usize,
        name: &'a str,
        inputs: &'a [Expression<F>],
        table: &'a [Expression<F>],
    ) -> Result<Self> {
        let (selectors, queries) = compute_gate_arity(inputs);
        let table_queries = compute_table_cells(table.iter())?;
        let kind = LookupKind::new(table, inputs)?;

        Ok(Self {
            kind,
            idx,
            name,
            inputs,
            table,
            table_queries,
            selectors,
            queries,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    fn module_name(&self) -> String {
        self.kind.module_name()
    }

    pub fn output_queries(&self) -> &[AnyQuery] {
        &self.table_queries
    }

    pub fn expressions(&self) -> impl Iterator<Item = (&Expression<F>, &Expression<F>)> {
        self.inputs.iter().zip(self.table)
    }

    pub fn kind(&self) -> &LookupKind {
        &self.kind
    }
}

/// Uniquely identifies a lookup target by the table columns required by it and its arity.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct LookupKind {
    columns: Vec<usize>,
    io: (usize, usize),
}

pub fn contains_fixed<F: Field>(e: &&Expression<F>) -> bool {
    fn false_cb<I>(_: I) -> bool {
        false
    }
    e.evaluate(
        &false_cb,
        &false_cb,
        &|_| true,
        &false_cb,
        &false_cb,
        &false_cb,
        &identity,
        &BitOr::bitor,
        &BitOr::bitor,
        &|b, _| b,
    )
}

impl LookupKind {
    /// Constructs a lookup kind. The columns are obtained from the tables array, which have to be
    /// expressions of type FixedQuery and the io is obtained from the inputs. Expressions that
    /// contain fixed columns are considered inputs while expressions that don't are considered
    /// outputs.
    pub fn new<F: Field>(
        tables: &[Expression<F>],
        inputs: &[Expression<F>],
    ) -> anyhow::Result<Self> {
        let columns = tables
            .iter()
            .map(|e| match e {
                Expression::Fixed(q) => Ok(q.column_index()),
                _ => anyhow::bail!("Unsupported table column definition: {e:?}"),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let (ins, outs) = inputs.iter().partition::<Vec<_>, _>(contains_fixed);
        Ok(Self {
            columns,
            io: (ins.len(), outs.len()),
        })
    }

    pub fn id(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }

    pub fn module_name(&self) -> String {
        format!("lookup_{}", self.id())
    }

    pub fn inputs(&self) -> usize {
        self.io.0
    }

    pub fn outputs(&self) -> usize {
        self.io.1
    }
}
