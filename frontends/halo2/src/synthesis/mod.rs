//! Defines types for handling the result of synthesizing a circuit.

use std::{collections::HashSet, convert::identity};

use anyhow::{Result, anyhow};
use constraint::{EqConstraint, EqConstraintArg, EqConstraintGraph};
use groups::{Group, GroupBuilder, GroupCell, Groups};
use regions::{FixedData, RegionIndexToStart, TableData};

use crate::{
    CircuitIO,
    gates::AnyQuery,
    halo2::{
        Field,
        groups::{GroupKey, RegionsGroup},
        *,
    },
    info_traits::{CSI, ConstraintSystemInfo, GateInfo},
    io::{AdviceIO, IOCell, InstanceIO},
    lookups::{Lookup, LookupTableRow},
    resolvers::FixedQueryResolver,
    value::steal,
};

pub mod constraint;
pub mod groups;
pub mod regions;

/// Result of synthesizing a circuit.
pub struct CircuitSynthesis<F>
where
    F: Field,
{
    id: usize,
    cs: CSI<F>,
    eq_constraints: EqConstraintGraph<F>,
    fixed: FixedData<F>,
    tables: Vec<TableData<F>>,
    groups: Groups,
}

impl<F> CircuitSynthesis<F>
where
    F: Field,
{
    /// Returns the list of gates in the constraint system.
    pub fn gates(&self) -> Vec<&dyn GateInfo<F>> {
        self.cs.gates()
    }

    /// Returns the lookups declared during synthesis.
    pub fn lookups<'a>(&'a self) -> Vec<Lookup<'a, F>> {
        Lookup::load(&*self.cs)
    }

    /// Finds the table that corresponds to the query set.
    fn find_table(&self, q: &[AnyQuery]) -> Result<Vec<Vec<F>>> {
        self.tables
            .iter()
            .find_map(|table| table.get_rows(q))
            .ok_or_else(|| anyhow!("Could not get values from table"))
            .and_then(identity)
    }

    /// Returns the list of tables the lookup refers to.
    pub fn tables_for_lookup(&self, l: &Lookup<F>) -> Result<Vec<LookupTableRow<F>>> {
        fn transpose<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
            assert!(!v.is_empty());
            let len = v[0].len();
            let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
            (0..len)
                .map(|_| {
                    iters
                        .iter_mut()
                        .map(|n| n.next().unwrap())
                        .collect::<Vec<T>>()
                })
                .collect()
        }

        l.table_queries().and_then(|q| {
            // For each table region look if they have the columns we are looking for and
            // collect all the fixed values
            let columns = q.iter().map(|q| q.column_index()).collect::<Vec<_>>();
            let table = self.find_table(&q)?;
            if q.len() != table.len() {
                anyhow::bail!(
                    "Inconsistency check failed: Lookup has {} columns but table yielded {}",
                    q.len(),
                    table.len()
                )
            }

            // The table needs to be transposed from [row,col] to [col,row].
            Ok(transpose(table)
                .into_iter()
                .map(|row| LookupTableRow::new(&columns, row))
                .collect())
        })
    }

    /// Returns the groups in the circuit.
    pub fn groups(&self) -> &Groups {
        &self.groups
    }

    /// Returns the equality constraints.
    pub fn constraints(&self) -> &EqConstraintGraph<F> {
        &self.eq_constraints
    }

    /// Returns a mapping from the region index to region start
    pub fn regions_by_index(&self) -> RegionIndexToStart {
        self.groups
            .as_ref()
            .iter()
            .flat_map(|g| g.regions())
            .enumerate()
            .map(|(idx, region)| (idx.into(), region.rows().start.into()))
            .collect()
    }

    /// Returns the top level group in the circuit.
    pub fn top_level_group(&self) -> Option<&Group> {
        self.groups.top_level()
    }

    /// Returns a reference to a resolver for fixed queries.
    pub fn fixed_query_resolver(&self) -> &dyn FixedQueryResolver<F> {
        &self.fixed
    }

    pub(crate) fn id(&self) -> usize {
        self.id
    }
}

impl<F: Field> std::fmt::Debug for CircuitSynthesis<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitSynthesis")
            .field("id", &self.id)
            .field("eq_constraints", &self.eq_constraints)
            .field("fixed", &self.fixed)
            .field("tables", &self.tables)
            .field("groups", &self.groups)
            .finish()
    }
}

/// Collects the information from the synthesis.
///
/// Use this struct to give information about the structure of the circuit during the call to
/// [`crate::CircuitCallbacks::synthesize`]. The synthesis of the circuit is divided in groups and
/// these are divided in regions. Groups can contain others groups inside them, forming a tree.
///
/// Before synthesis a default top-level group is initialized such that you don't need to do
/// anything with them if your use case doesn't need dividing the circuit into groups.
///
/// The circuit is represented as a table of cells divided into regions. The region boundaries are
/// represented by a set of columns and an interval of the rows of the table. Regions cannot
/// overlap and during synthesis there can only be one active region that must be exited before
/// opening a new one.
///
/// Regions can represent lookup tables and these can only contain fixed columns.
///
/// Regions also have a set of selectors that can be turned on per row of the region. These
/// selectors are used to check what polynomials returned by the [`GateInfo`] instances are
/// enabled in that row. The driver will emit IR for each polynomial that is enabled in each row of
/// each region.
pub struct Synthesizer<F: Field> {
    // Unique identifier wrt a driver instance for this synthesis process.
    id: usize,
    // Keeps track of the construction of the groups tree.
    groups: GroupBuilder,
    // Data for the columns containing fixed values.
    fixed: FixedData<F>,
    // Undirected graph of equality constraints between cells in the table.
    eq_constraints: EqConstraintGraph<F>,
    // A list of set of columns. Represents the regions that need to be converted into tables.
    // After a region has finished processing, if it was marked as a table the information about
    // it in the regions list is discarded and the set of columns that comprise the table is moved
    // to this list.
    tables: Vec<HashSet<Column<Fixed>>>,
    // This iterator yields indices for the regions inside the circuit. Each region has an unique
    // index. Regions marked as tables discard their index, that is reused for the next
    // region.
    next_index: Box<dyn Iterator<Item = RegionIndex>>,
}

impl<F: Field> Synthesizer<F> {
    pub(crate) fn new(id: usize) -> Self {
        Self {
            id,
            groups: Default::default(),
            fixed: Default::default(),
            eq_constraints: Default::default(),
            tables: Default::default(),
            next_index: Box::new((0..).map(RegionIndex::from)),
        }
    }

    /// Configures the IO of the circuit.
    pub(crate) fn configure_io(&mut self, advice_io: AdviceIO, instance_io: InstanceIO) {
        add_root_io(&mut self.groups, &advice_io);
        add_root_io(&mut self.groups, &instance_io);
    }

    /// Builds a [`CircuitSynthesis`] with the information recollected about the circuit.
    pub(crate) fn build<CS>(mut self, cs: CS) -> Result<CircuitSynthesis<F>>
    where
        CS: ConstraintSystemInfo<F> + 'static,
    {
        add_fixed_to_const_constraints(&mut self.eq_constraints, &self.fixed)?;

        Ok(CircuitSynthesis {
            id: self.id,
            cs: Box::new(cs),
            eq_constraints: self.eq_constraints,
            tables: fill_tables(self.tables, &self.fixed)?,
            fixed: self.fixed,
            groups: self.groups.into_root().flatten(),
        })
    }

    /// Enters a new region of the circuit.
    ///
    /// Panics if the synthesizer entered a region already and didn't exit.
    pub fn enter_region(&mut self, region_name: String) {
        self.groups
            .regions_mut()
            .push(|| region_name, &mut self.next_index, &mut self.tables);
    }

    /// Exits the current region of the circuit.
    ///
    /// Panics if the synthesizer didn't entered a region prior.
    pub fn exit_region(&mut self) {
        self.groups.regions_mut().commit();
    }

    /// Marks the given selector as enabled for the table row.
    pub fn enable_selector(&mut self, selector: Selector, row: usize) {
        self.groups.regions_mut().edit(|region| {
            region.enable_selector(selector, row);
        });
    }

    /// Process that inside the entered region the circuit assigned a value to an advice cell.
    pub fn on_advice_assigned(&mut self, advice: Column<Advice>, row: usize) {
        self.groups.regions_mut().edit(|region| {
            region.update_extent(advice.into(), row);
        });
    }

    /// Process that inside the entered region the circuit assigned a value to a fixed cell.
    pub fn on_fixed_assigned(&mut self, fixed: Column<Fixed>, row: usize, value: F) {
        // Assignments to fixed cells can happen outside a region so we write those on the last
        // region if available
        self.groups.regions_mut().edit(|region| {
            region.update_extent(fixed.into(), row);
        });
        self.fixed.assign_fixed(fixed, row, Value::known(value));
    }

    /// Annotates that the two given cells have a copy constraint between them.
    pub fn copy(&mut self, from: Column<Any>, from_row: usize, to: Column<Any>, to_row: usize) {
        self.eq_constraints
            .add(EqConstraint::AnyToAny(from, from_row, to, to_row));
    }

    /// Annotates that starting from the given row the given fixed column has that value.
    pub fn fill_from_row(&mut self, column: Column<Fixed>, row: usize, value: F) {
        log::debug!("fill_from_row{:?}", (column, row, value));
        self.fixed.blanket_fill(column, row, Value::known(value));
        let r = self.groups.regions_mut();
        r.edit(|region| region.update_extent(column.into(), row));
    }

    /// Annotates that starting from the given row the given fixed column has that value and marks
    /// the current region as a table.
    pub fn fill_table(&mut self, column: Column<Fixed>, row: usize, value: F) {
        self.fill_from_row(column, row, value);
        self.mark_region_as_table();
    }

    /// Marks the current region as a table.
    pub fn mark_region_as_table(&mut self) {
        self.groups.regions_mut().mark_region()
    }

    /// Pushes a new namespace.
    pub fn push_namespace(&mut self, name: String) {
        self.groups
            .regions_mut()
            .edit(|region| region.push_namespace(|| name));
    }

    /// Pops the most recent namespace.
    pub fn pop_namespace(&mut self, name: Option<String>) {
        self.groups
            .regions_mut()
            .edit(|region| region.pop_namespace(name));
    }

    /// Enters a new group, pushing it to the top of the stack.
    ///
    /// This group is then the new active group.
    pub fn enter_group<K>(&mut self, name: String, key: K)
    where
        K: GroupKey,
    {
        log::debug!("Entering group '{name}'");
        self.groups.push(|| name, key)
    }

    /// Pops the active group from the stack and marks it as a children of the next group.
    ///
    /// The next group becomes the new active group.
    ///
    /// Panics if attempted to pop a group without pushing one prior.
    pub fn exit_group(&mut self, meta: RegionsGroup) {
        for input in meta.inputs() {
            self.groups.add_input(input);
        }
        for output in meta.outputs() {
            self.groups.add_output(output);
        }
        log::debug!("Exiting group '{}'", self.groups.current().name());
        self.groups.pop();
    }
}

impl<F: Field> std::fmt::Debug for Synthesizer<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Synthesizer")
            .field("id", &self.id)
            .field("groups", &self.groups)
            .field("fixed", &self.fixed)
            .field("eq_constraints", &self.eq_constraints)
            .field("tables", &self.tables)
            .finish()
    }
}

/// Create TableData structures for lookup tables
fn fill_tables<F: Field>(
    tables: Vec<HashSet<Column<Fixed>>>,
    fixed: &FixedData<F>,
) -> Result<Vec<TableData<F>>> {
    tables
        .into_iter()
        .map(|set| fixed.subset(set).map(TableData::new))
        .collect()
}

/// Add edges in the graph from fixed cells to their assigned values.
fn add_fixed_to_const_constraints<F: Field>(
    constraints: &mut EqConstraintGraph<F>,
    fixed: &FixedData<F>,
) -> Result<()> {
    let fixed_cells = {
        constraints.vertices().into_iter().filter_map(|v| match v {
            EqConstraintArg::Any(col, row) => {
                let col: Option<Column<Fixed>> = col.try_into().ok();
                col.map(|col| (col, row))
            }
            _ => None,
        })
    };

    for (col, row) in fixed_cells {
        let value = steal(&fixed.resolve_fixed(col.index(), row))
            .ok_or_else(|| anyhow!("Fixed cell was assigned an unknown value!"))?;
        constraints.add(EqConstraint::FixedToConst(col, row, value));
    }

    Ok(())
}

/// Adds to the list of input and output cells of the top-level block.
fn add_root_io<C: ColumnType>(groups: &mut GroupBuilder, io: &CircuitIO<C>)
where
    IOCell<C>: Into<GroupCell>,
{
    for c in io.inputs() {
        groups.add_root_input(*c);
    }

    for c in io.outputs() {
        groups.add_root_output(*c);
    }
}
