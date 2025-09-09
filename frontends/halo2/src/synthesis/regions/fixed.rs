use std::collections::{HashMap, HashSet};

use crate::{
    backend::resolvers::FixedQueryResolver,
    halo2::{Assigned, Column, Field, Fixed, FixedQuery, Value},
    value::steal,
};

use super::BlanketFills;

#[derive(Default, Debug, Clone)]
pub struct FixedData<F: Copy + std::fmt::Debug + Default> {
    /// Constant values assigned to fixed columns in the region.
    fixed: HashMap<usize, HashMap<usize, Value<F>>>,
    /// Set of columns for which there is data.
    columns: HashSet<Column<Fixed>>,
    /// Represents the circuit filling rows with a single value.
    /// Row start offsets are maintained in chronological order, so when
    /// querying a row the latest that matches is the correct value.
    blanket_fills: HashMap<usize, BlanketFills<F>>,
}

pub type FixedAssigned<F> = HashMap<(usize, usize), Value<F>>;
pub type FixedBlanket<F> = HashMap<usize, BlanketFills<F>>;

impl<F: Copy + std::fmt::Debug + Default> FixedData<F> {
    pub fn take(self) -> (FixedAssigned<F>, FixedBlanket<F>) {
        (
            self.fixed
                .into_iter()
                .flat_map(|(col, values)| {
                    values
                        .into_iter()
                        .map(move |(row, value)| ((col, row), value))
                })
                .collect(),
            self.blanket_fills,
        )
    }

    pub fn blanket_fill(&mut self, column: Column<Fixed>, row: usize, value: Value<F>) {
        self.columns.insert(column);
        self.blanket_fills
            .entry(column.index())
            .or_default()
            .push((row.., value));
    }

    pub fn assign_fixed<VR>(&mut self, fixed: Column<Fixed>, row: usize, value: Value<VR>)
    where
        F: Field,
        VR: Into<Assigned<F>>,
    {
        let value = value.map(|vr| vr.into());
        log::debug!(
            "Recording fixed assignment @ col = {}, row = {row}, value = {value:?}",
            fixed.index()
        );
        self.columns.insert(fixed);
        self.fixed
            .entry(fixed.index())
            .or_default()
            .insert(row, value.map(|vr| vr.evaluate()));
    }

    fn resolve_from_blanket_fills(&self, column: usize, row: usize) -> Option<Value<F>>
    where
        F: Field,
    {
        self.blanket_fills
            .get(&column)
            .and_then(|values| values.iter().rfind(|(range, _)| range.contains(&row)))
            .map(|(_, v)| *v)
    }

    pub fn resolve_fixed(&self, column: usize, row: usize) -> Value<F>
    where
        F: Field,
    {
        self.fixed
            .get(&column)
            .and_then(|cols| cols.get(&row))
            .inspect(|v| log::debug!(" For ({column}, {row}) we got value {v:?}",))
            .cloned()
            .or_else(|| self.resolve_from_blanket_fills(column, row))
            // Default to zero if all else fails
            .unwrap_or(Value::known(F::ZERO))
    }

    /// Returns a copy of itself by selecting only the given columns.
    ///
    /// If a column is not in the fixed data returns an error.
    pub fn subset(&self, columns: HashSet<Column<Fixed>>) -> anyhow::Result<Self> {
        let mut selected = Self::default();
        if !self.columns.is_superset(&columns) {
            anyhow::bail!("Fixed data does not have all the required columns.")
        }
        selected.columns = columns;
        for col in &selected.columns {
            if let Some(fill) = self.blanket_fills.get(&col.index()) {
                selected.blanket_fills.insert(col.index(), fill.clone());
            }
            if let Some(values) = self.fixed.get(&col.index()) {
                selected.fixed.insert(col.index(), values.clone());
            }
        }

        Ok(selected)
    }
}

impl<F: Field> FixedQueryResolver<F> for FixedData<F> {
    fn resolve_query(&self, query: &FixedQuery, row: usize) -> anyhow::Result<F> {
        let value = self.resolve_fixed(query.column_index(), row);

        steal(&value).ok_or_else(|| anyhow::anyhow!("Fixed cell was assigned an unknown value!"))
    }
}
