//! Synthesis related traits.

use ff::Field;

use crate::{
    info_traits::{GroupInfo, SelectorInfo},
    query::{Advice, Fixed},
    table::{Any, Column},
};

/// Unique identifier for a group.
pub type GroupKey = u64;

/// Defines the behavior of a synthesizer.
///
/// This crate doesn't include an implementation of this trait.
pub trait SynthesizerLike<F: Field> {
    /// Enters a new region of the circuit.
    ///
    /// Panics if the synthesizer entered a region already and didn't exit.
    fn enter_region(&mut self, region_name: String);

    /// Exits the current region of the circuit.
    ///
    /// Panics if the synthesizer didn't entered a region prior.
    fn exit_region(&mut self);

    /// Marks the given selector as enabled for the table row.
    fn enable_selector(&mut self, selector: &dyn SelectorInfo, row: usize);

    /// Process that inside the entered region the circuit assigned a value to an advice cell.
    fn on_advice_assigned(&mut self, advice: impl Into<Column<Advice>>, row: usize);

    /// Process that inside the entered region the circuit assigned a value to a fixed cell.
    fn on_fixed_assigned(&mut self, fixed: impl Into<Column<Fixed>>, row: usize, value: F);

    /// Annotates that the two given cells have a copy constraint between them.
    fn copy(
        &mut self,
        from: impl Into<Column<Any>>,
        from_row: usize,
        to: impl Into<Column<Any>>,
        to_row: usize,
    );

    /// Annotates that starting from the given row the given fixed column has that value.
    fn fill_from_row(&mut self, column: impl Into<Column<Fixed>>, row: usize, value: F);

    /// Annotates that starting from the given row the given fixed column has that value and marks
    /// the current region as a table.
    fn fill_table(&mut self, column: impl Into<Column<Fixed>>, row: usize, value: F) {
        self.fill_from_row(column, row, value);
        self.mark_region_as_table();
    }

    /// Marks the current region as a table.
    fn mark_region_as_table(&mut self);

    /// Pushes a new namespace.
    fn push_namespace(&mut self, name: String);

    /// Pops the most recent namespace.
    fn pop_namespace(&mut self, name: Option<String>);

    /// Enters a new group, pushing it to the top of the stack.
    ///
    /// This group is then the new active group.
    fn enter_group(&mut self, name: String, key: impl Into<GroupKey>);

    /// Pops the active group from the stack and marks it as a children of the next group.
    ///
    /// The next group becomes the new active group.
    ///
    /// Panics if attempted to pop a group without pushing one prior.
    fn exit_group(&mut self, meta: impl GroupInfo);
}
