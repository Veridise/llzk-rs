use crate::{gates::AnyQuery, halo2::Value, value::steal};
use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap, HashSet},
    convert::identity,
    ops::RangeFrom,
};

use super::BlanketFills;

#[derive(Clone, Debug)]
pub enum Fill<F: Copy> {
    Single(usize, Value<F>),
    Many(RangeFrom<usize>, Value<F>),
}

impl<F: Copy> Fill<F> {
    pub fn row(&self) -> usize {
        match self {
            Fill::Single(row, _) => *row,
            Fill::Many(range_from, _) => range_from.start,
        }
    }
}

impl<F: Copy> PartialEq for Fill<F> {
    fn eq(&self, other: &Self) -> bool {
        self.row() == other.row()
    }
}

impl<F: Copy> Eq for Fill<F> {}

impl<F: Copy> PartialOrd for Fill<F> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.row().partial_cmp(&other.row())
    }
}

impl<F: Copy> Ord for Fill<F> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.row().cmp(&other.row())
    }
}

#[derive(Debug)]
pub struct TableData<F: Copy> {
    values: HashMap<usize, Vec<Fill<F>>>,
}

pub enum ColumnMatch {
    Contained,
    Missing(Vec<AnyQuery>),
}

impl<F: Copy + Default> TableData<F> {
    pub fn new(
        fixed: HashMap<(usize, usize), Value<F>>,
        blanket_fills: HashMap<usize, BlanketFills<F>>,
    ) -> Self {
        let values = fixed
            .into_iter()
            .map(|((col, row), v)| (col, Fill::Single(row, v)))
            .chain(blanket_fills.into_iter().flat_map(|(col, blanket)| {
                blanket
                    .into_iter()
                    .map(move |(r, v)| (col, Fill::Many(r, v)))
            }))
            .fold(
                HashMap::<usize, Vec<Fill<F>>>::new(),
                |mut map, (col, fill)| {
                    map.entry(col).or_default().push(fill);
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

    fn get_rows_impl(&self, cols: &[AnyQuery]) -> anyhow::Result<Vec<Vec<F>>> {
        let tables = cols
            .iter()
            .map(|c| self.values[&c.column_index()].as_slice())
            .collect::<Vec<_>>();

        let upper_limit = tables
            .iter()
            .map(|table| {
                let (singles, blankets) = table
                    .iter()
                    .partition::<Vec<_>, _>(|f| matches!(f, Fill::Single(_, _)));
                let largest_single: Option<&Fill<_>> = singles.into_iter().max();
                let largest_blanket: Option<&Fill<_>> = blankets.into_iter().max();

                match (largest_single, largest_blanket) {
                    (None, None) => None,
                    (Some(s), None) => Some(s),
                    (_, Some(b)) => Some(b),
                }
            })
            .try_fold(None, |acc: Option<Vec<&Fill<F>>>, f| {
                Ok(match f {
                    Some(f) => match acc {
                        Some(mut accs) => {
                            for acc in &accs {
                                if match (acc, f) {
                                    (Fill::Single(acc, _), Fill::Single(row, _)) => acc != row,

                                    (Fill::Single(acc, _), Fill::Many(range_from, _)) => {
                                        range_from.contains(acc)
                                    }

                                    (Fill::Many(acc, _), Fill::Single(row, _)) => acc.contains(row),
                                    (Fill::Many(acc, _), Fill::Many(range_from, _)) => {
                                        acc.contains(&range_from.start)
                                            || range_from.contains(&acc.start)
                                    }
                                } {
                                    anyhow::bail!("Wrong table size for columns")
                                }
                            }
                            accs.push(f);
                            Some(accs)
                        }
                        None => Some(vec![f]),
                    },
                    None => anyhow::bail!("Could not get the largest row fill of table"),
                })
            })
            .and_then(|v| {
                v.ok_or_else(|| anyhow::anyhow!("Could not get the largest row fill of table"))
            })?
            .into_iter()
            .max()
            .ok_or_else(|| anyhow::anyhow!("Could not get the largest row fill of table"))?
            .row();

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
    table: &[Fill<F>],
    upper_limit: usize,
) -> anyhow::Result<Vec<Value<F>>> {
    let mut dense = vec![Default::default(); upper_limit + 1];
    let mut check = vec![false; upper_limit + 1];

    let (singles, blankets) = table
        .iter()
        .partition::<Vec<_>, _>(|f| matches!(f, Fill::Single(_, _)));
    for blanket in blankets {
        match blanket {
            Fill::Many(range, value) => {
                for idx in range.start..=upper_limit {
                    dense[idx] = *value;
                    check[idx] = true;
                }
            }
            _ => unreachable!(),
        }
    }

    // Singles last for overriding the blankets
    for single in singles {
        match single {
            Fill::Single(row, value) => {
                dense[*row] = *value;
                check[*row] = true;
            }
            _ => unreachable!(),
        }
    }

    if !check.into_iter().all(identity) {
        anyhow::bail!("Detected gaps in table!")
    }

    Ok(dense)
}
