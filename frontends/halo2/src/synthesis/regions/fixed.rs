use std::{collections::HashMap, ops::AddAssign};

use crate::halo2::{Assigned, Column, Field, Fixed, Value};

use super::BlanketFills;

#[derive(Default, Debug, Clone)]
pub struct FixedData<F: Copy + std::fmt::Debug> {
    /// Constant values assigned to fixed columns in the region.
    fixed: HashMap<(usize, usize), Value<F>>,
    /// Represents the circuit filling rows with a single value.
    /// Row start offsets are maintained in chronological order, so when
    /// querying a row the latest that matches is the correct value.
    blanket_fills: HashMap<usize, BlanketFills<F>>,
}

impl<F: Copy + std::fmt::Debug> FixedData<F> {
    pub fn take(
        self,
    ) -> (
        HashMap<(usize, usize), Value<F>>,
        HashMap<usize, BlanketFills<F>>,
    ) {
        (self.fixed, self.blanket_fills)
    }

    pub fn blanket_fill(&mut self, column: Column<Fixed>, row: usize, value: Value<F>) {
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
        self.fixed
            .insert((fixed.index(), row), value.map(|vr| vr.evaluate()));
    }

    fn resolve_from_blanket_fills(&self, column: usize, row: usize) -> Option<Value<F>> {
        self.blanket_fills
            .get(&column)
            .and_then(|values| values.iter().rfind(|(range, _)| range.contains(&row)))
            .map(|(_, v)| *v)
    }

    pub fn resolve_fixed(&self, column: usize, row: usize) -> Option<Value<F>> {
        self.fixed
            .get(&(column, row))
            .inspect(|v| log::debug!(" For ({column}, {row}) we got value {v:?}",))
            .cloned()
            .or_else(|| self.resolve_from_blanket_fills(column, row))
    }
}

impl<F: Copy + std::fmt::Debug> AddAssign for FixedData<F> {
    fn add_assign(&mut self, rhs: Self) {
        self.fixed.extend(rhs.fixed);
        self.blanket_fills.extend(rhs.blanket_fills);
    }
}
