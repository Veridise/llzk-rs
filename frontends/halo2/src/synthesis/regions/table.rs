use crate::{gates::AnyQuery, halo2::Value, value::steal};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    convert::identity,
};

use super::fixed::FixedData;

/// Key used to represent filled rows in the column
#[derive(Copy, Clone, Debug)]
pub enum Fill {
    Single(usize),
    Many(usize),
}

impl Fill {
    pub fn row(&self) -> usize {
        match self {
            Fill::Single(row) => *row,
            Fill::Many(from) => *from,
        }
    }
}

impl PartialEq for Fill {
    fn eq(&self, other: &Self) -> bool {
        self.row() == other.row()
    }
}

impl Eq for Fill {}

impl PartialOrd for Fill {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.row().partial_cmp(&other.row())
    }
}

impl Ord for Fill {
    fn cmp(&self, other: &Self) -> Ordering {
        self.row().cmp(&other.row())
    }
}

/// Sparse representation of a table.
#[derive(Debug)]
pub struct TableData<F: Copy> {
    values: HashMap<usize, BTreeMap<Fill, Value<F>>>,
}

pub enum ColumnMatch {
    Contained,
    Missing(Vec<AnyQuery>),
}

impl<F: Copy + Default + std::fmt::Debug> TableData<F> {
    pub fn new(fixed: FixedData<F>) -> Self {
        let (fixed, blanket_fills) = fixed.take();
        let values = fixed
            .into_iter()
            .map(|((col, row), v)| (col, (Fill::Single(row), v)))
            .chain(blanket_fills.into_iter().flat_map(|(col, blanket)| {
                blanket
                    .into_iter()
                    .map(move |(r, v)| (col, (Fill::Many(r.start), v)))
            }))
            .fold(
                HashMap::<usize, BTreeMap<Fill, Value<F>>>::new(),
                |mut map, (col, (fill, value))| {
                    map.entry(col).or_default().insert(fill, value);
                    map
                },
            );
        Self { values }
    }

    pub fn check_columns(&self, cols: &[AnyQuery]) -> ColumnMatch {
        assert!(!cols.is_empty());
        let col_set = self.collect_columns();
        cols.iter()
            .map(|q| {
                if col_set.contains(&q.column_index()) {
                    ColumnMatch::Contained
                } else {
                    ColumnMatch::Missing(vec![q.clone()])
                }
            })
            .fold(ColumnMatch::Contained, |acc, m| match (acc, m) {
                (ColumnMatch::Contained, ColumnMatch::Contained) => ColumnMatch::Contained,
                (ColumnMatch::Contained, ColumnMatch::Missing(items)) => {
                    ColumnMatch::Missing(items)
                }
                (ColumnMatch::Missing(items), ColumnMatch::Contained) => {
                    ColumnMatch::Missing(items)
                }
                (ColumnMatch::Missing(mut acc), ColumnMatch::Missing(q)) => {
                    acc.extend(q);
                    ColumnMatch::Missing(acc)
                }
            })
    }

    fn find_upper_limit(&self, tables: &[&BTreeMap<Fill, Value<F>>]) -> anyhow::Result<usize> {
        tables
            .iter()
            .map(|table| {
                table
                    .keys()
                    .rev()
                    .find(|k| matches!(k, Fill::Single(_)))
                    .or_else(|| table.keys().rev().find(|k| !matches!(k, Fill::Single(_))))
            })
            .collect::<Option<Vec<_>>>()
            .and_then(|upper_limits| upper_limits.into_iter().max())
            .map(|f| f.row())
            .ok_or_else(|| anyhow::anyhow!("Could not get the largest row fill of table"))
    }

    fn get_rows_impl(&self, cols: &[AnyQuery]) -> anyhow::Result<Vec<Vec<F>>> {
        let tables = cols
            .iter()
            .map(|c| &self.values[&c.column_index()])
            .collect::<Vec<_>>();

        let upper_limit = self.find_upper_limit(&tables)?;

        tables
            .into_iter()
            .map(|table| {
                fill_table(table, upper_limit).and_then(|t| {
                    t.into_iter()
                        .map(|v| {
                            steal(&v).ok_or_else(|| {
                                anyhow::anyhow!("Table value filled with unknown value!")
                            })
                        })
                        .collect::<anyhow::Result<Vec<_>>>()
                })
            })
            .collect()
    }

    /// Returns the rows the table has for the given columns. The columns have to have
    /// the same number of rows
    pub fn get_rows(&self, cols: &[AnyQuery]) -> Option<anyhow::Result<Vec<Vec<F>>>> {
        if matches!(self.check_columns(cols), ColumnMatch::Missing(_)) {
            return None;
        }
        Some(self.get_rows_impl(cols))
    }

    fn collect_columns(&self) -> HashSet<usize> {
        self.values.keys().copied().collect()
    }
}

fn fill_table<F: Default + Copy>(
    table: &BTreeMap<Fill, Value<F>>,
    upper_limit: usize,
) -> anyhow::Result<Vec<Value<F>>> {
    let mut dense = vec![Default::default(); upper_limit + 1];
    let mut check = vec![false; upper_limit + 1];

    let mut last = upper_limit + 1;
    for (fill, value) in table
        .iter()
        .rev()
        .filter(|(f, _)| matches!(f, Fill::Many(_)))
    {
        for idx in fill.row()..last {
            dense[idx] = *value;
            check[idx] = true;
        }
        last = fill.row();
    }
    // Singles last for overriding the blankets
    for (fill, value) in table.iter().filter(|(f, _)| matches!(f, Fill::Single(_))) {
        dense[fill.row()] = *value;
        check[fill.row()] = true;
    }

    if !check.into_iter().all(identity) {
        anyhow::bail!("Detected gaps in table!")
    }

    Ok(dense)
}
