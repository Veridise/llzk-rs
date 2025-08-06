use std::{
    hash::{DefaultHasher, Hash as _, Hasher as _},
    ops::Index,
};

use crate::{
    backend::codegen::lookup::query_from_table_expr,
    gates::{compute_gate_arity, AnyQuery},
    halo2::{Expression, Field, FixedQuery, Selector},
    synthesis::CircuitSynthesis,
};
use anyhow::{bail, Result};

pub mod callbacks;

/// Lightweight representation of a lookup that is cheap to copy
#[derive(Clone, Copy)]
pub struct Lookup<'a, F: Field> {
    name: &'a str,
    idx: usize,
    inputs: &'a [Expression<F>],
    table: &'a [Expression<F>],
}

/// A heavier representation of the lookup
#[derive(Clone)]
pub struct LookupData<'a, F: Field> {
    lookup: Lookup<'a, F>,
    selectors: Vec<&'a Selector>,
    queries: Vec<AnyQuery>,
    table_queries: Vec<AnyQuery>,
    kind: LookupKind,
}

impl<'a, F: Field> TryFrom<Lookup<'a, F>> for LookupData<'a, F> {
    type Error = anyhow::Error;

    fn try_from(lookup: Lookup<'a, F>) -> std::result::Result<Self, Self::Error> {
        let (selectors, queries) = compute_gate_arity(lookup.inputs);
        let table_queries = compute_table_cells(lookup.table.iter())?;
        let kind = lookup.kind()?;
        Ok(LookupData {
            lookup,
            selectors,
            queries,
            table_queries,
            kind,
        })
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
    pub fn load(syn: &'a CircuitSynthesis<F>) -> Vec<Self> {
        syn.cs()
            .lookups()
            .iter()
            .enumerate()
            .map(|(idx, a)| {
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
    ) -> Self {
        Self {
            idx,
            name,
            inputs,
            table,
        }
    }

    //pub fn callbacks(&self) -> &dyn LookupCallbacks<F> {
    //    self.callbacks
    //}

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn module_name(&self) -> Result<String> {
        self.kind().map(|kind| kind.module_name())
    }

    pub fn expressions(&self) -> impl Iterator<Item = (&'a Expression<F>, &'a Expression<F>)> + 'a {
        self.inputs.iter().zip(self.table)
    }

    pub fn inputs(&self) -> &'a [Expression<F>] {
        self.inputs
    }

    pub fn kind(&self) -> Result<LookupKind> {
        LookupKind::new(self.table, self.inputs)
    }

    pub fn table_queries(&self) -> Result<Vec<AnyQuery>> {
        compute_table_cells(self.table.iter())
    }

    pub fn expr_for_column(&self, col: usize) -> Result<&Expression<F>> {
        self.table_queries()?
            .into_iter()
            .enumerate()
            .find(|(_, q)| q.column_index() == col)
            .ok_or_else(|| anyhow::anyhow!("Column {col} not found"))
            .map(|(idx, _)| &self.inputs[idx])
    }
}

impl<'a, F: Field> LookupData<'a, F> {
    pub fn output_queries(&self) -> &[AnyQuery] {
        &self.table_queries
    }
}

/// Uniquely identifies a lookup target by the table columns used in it.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct LookupKind {
    columns: Vec<usize>,
    //io: (usize, usize),
}

#[inline]
fn zip_res<L, R, E>(lhs: Result<L, E>, rhs: Result<R, E>) -> Result<(L, R), E> {
    lhs.and_then(|lhs| rhs.map(|rhs| (lhs, rhs)))
}

impl LookupKind {
    /// Constructs a lookup kind. The callbacks are invoked for deducing which columns are
    /// designated as inputs and which as outputs.
    pub fn new<F: Field>(
        tables: &[Expression<F>],
        inputs: &[Expression<F>],
        //lookups: &dyn LookupCallbacks<F>,
    ) -> anyhow::Result<Self> {
        fn empty_io() -> anyhow::Result<(usize, usize)> {
            Ok((0, 0))
        }
        let columns = tables
            .iter()
            .map(|e| match e {
                Expression::Fixed(q) => Ok(q.column_index()),
                _ => bail!("Unsupported table column definition: {e:?}"),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        //let io = inputs
        //    .into_iter()
        //    .zip(columns.iter().copied())
        //    .fold(HashMap::<usize, Vec<_>>::default(), |mut map, (e, t)| {
        //        map.entry(t).or_default().push(e);
        //        map
        //    })
        //    .into_iter()
        //    .map(|(t, exprs)| {
        //        exprs
        //            .into_iter()
        //            .map(|e| lookups.assign_io_kind(e, t))
        //            .map(Ok)
        //            .reduce(|lhs, rhs| {
        //                zip_res(lhs, rhs).and_then(|(lhs, rhs)| {
        //                    if lhs == rhs {
        //                        bail!("Column {t} assigned different IO types")
        //                    }
        //                    Ok(lhs)
        //                })
        //            })
        //            .ok_or_else(|| anyhow!("No expressions for column {t}"))
        //            .and_then(identity)
        //    })
        //    .try_fold((0, 0), |(i, o), io| -> anyhow::Result<(usize, usize)> {
        //        Ok(match io? {
        //            LookupIO::I => (i + 1, o),
        //            LookupIO::O => (i, o + 1),
        //        })
        //    })?;
        //
        //if io.1 == 0 {
        //    bail!("Lookup has to have at least one output!");
        //}
        Ok(Self { columns })
    }

    pub fn id(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }

    pub fn module_name(&self) -> String {
        format!("lookup_{}", self.id())
    }

    pub fn columns(&self) -> &[usize] {
        &self.columns
    }

    //pub fn inputs(&self) -> usize {
    //    self.io.0
    //}
    //
    //pub fn outputs(&self) -> usize {
    //    self.io.1
    //}
}

/// Represents a row in the lookup table that can be indexed by the columns participating in the
/// lookup.
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
        let index = self.col_to_index(index).expect(
            format!(
                "Can't index with a column outside of the valid range {:?}",
                self.columns
            )
            .as_str(),
        );
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
