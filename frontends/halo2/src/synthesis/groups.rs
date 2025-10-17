use std::{borrow::Borrow, collections::HashMap, ops::Deref};

use crate::{
    adaptors::GateAdaptor,
    halo2::{
        Advice, Any, Cell, Column, ColumnType, Expression, Field, Instance, RegionIndex, Rotation,
        groups::{GroupKey, GroupKeyInstance},
    },
    io::{AdviceIO, IOCell, InstanceIO},
    lookups::Lookup,
    resolvers::FixedQueryResolver,
};

use super::regions::{RegionData, RegionRow, Regions};

/// A group can either represent the circuit itself (the top level)
/// or a group declared during synthesis, identified by its key.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupKind {
    TopLevel,
    Group(GroupKeyInstance),
}

/// A cell that could be either assigned during synthesis or declared as circuit IO.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GroupCell {
    /// A cell that comes from an assigned cell during synthesis.
    Assigned(Cell),
    /// An instance cell that was declared as part of the circuit's IO.
    InstanceIO(IOCell<Instance>),
    /// An advice cell that was declared as part of the circuit's IO.
    AdviceIO(IOCell<Advice>),
}

impl GroupCell {
    pub fn to_expr<F: Field>(self) -> Expression<F> {
        match self {
            GroupCell::Assigned(cell) => cell.column.query_cell(Rotation::cur()),
            GroupCell::InstanceIO(cell) => cell.0.query_cell(Rotation::cur()),
            GroupCell::AdviceIO(cell) => cell.0.query_cell(Rotation::cur()),
        }
    }

    pub fn row(&self) -> usize {
        match self {
            GroupCell::Assigned(cell) => cell.row_offset,
            GroupCell::InstanceIO(cell) => cell.1,
            GroupCell::AdviceIO(cell) => cell.1,
        }
    }

    pub fn region_index(&self) -> Option<RegionIndex> {
        match self {
            GroupCell::Assigned(cell) => Some(cell.region_index),
            _ => None,
        }
    }

    pub fn col(&self) -> Column<Any> {
        match self {
            GroupCell::Assigned(cell) => cell.column,
            GroupCell::InstanceIO((col, _)) => (*col).into(),
            GroupCell::AdviceIO((col, _)) => (*col).into(),
        }
    }

    /// Tries to construct a a cell from a tuple of column and row.
    ///
    /// If the column is Fixed returns None.
    pub fn from_tuple((col, row): (Column<Any>, usize)) -> Option<Self> {
        if let Ok(col) = Column::<Instance>::try_from(col) {
            return Some(Self::InstanceIO((col, row)));
        }
        Column::<Advice>::try_from(col)
            .ok()
            .map(|col| Self::AdviceIO((col, row)))
    }

    /// Returns true if the cell is from a Fixed column.
    pub fn is_fixed(&self) -> bool {
        match self {
            GroupCell::Assigned(cell) => *cell.column.column_type() == Any::Fixed,
            GroupCell::InstanceIO(_) | GroupCell::AdviceIO(_) => false,
        }
    }
}

impl From<Cell> for GroupCell {
    fn from(value: Cell) -> Self {
        Self::Assigned(value)
    }
}

impl From<IOCell<Instance>> for GroupCell {
    fn from(value: IOCell<Instance>) -> Self {
        Self::InstanceIO(value)
    }
}

impl From<IOCell<Advice>> for GroupCell {
    fn from(value: IOCell<Advice>) -> Self {
        Self::AdviceIO(value)
    }
}

/// A flat read-only representation of a group.
///
/// The parent-children relation is represented by indices on a vector instead.
#[derive(Debug)]
pub struct Group {
    kind: GroupKind,
    name: Option<String>,
    inputs: Vec<GroupCell>,
    outputs: Vec<GroupCell>,
    regions: Regions,
    children: Vec<usize>,
}

impl Group {
    fn new(
        kind: GroupKind,
        name: Option<String>,
        inputs: Vec<GroupCell>,
        outputs: Vec<GroupCell>,
        regions: Regions,
        children: Vec<usize>,
    ) -> Self {
        // Sanity check; if group is top level there cannot be any assigned cells.
        if kind == GroupKind::TopLevel {
            assert!(
                inputs.iter().all(|i| !matches!(i, GroupCell::Assigned(_))),
                "Cannot assign input cells in the top level"
            );
            assert!(
                outputs.iter().all(|i| !matches!(i, GroupCell::Assigned(_))),
                "Cannot assign output cells in the top level"
            );
        }
        Self {
            kind,
            name,
            inputs,
            outputs,
            regions,
            children,
        }
    }

    /// Returns a list of region data.
    pub fn regions<'a>(&'a self) -> Vec<RegionData<'a>> {
        self.regions.regions()
    }

    /// Returns the regions' rows
    pub fn region_rows<'a, 'io, 'fq, F: Field>(
        &'a self,
        advice_io: &'io AdviceIO,
        instance_io: &'io InstanceIO,
        fqr: &'fq dyn FixedQueryResolver<F>,
    ) -> Vec<RegionRow<'a, 'io, 'fq, F>> {
        self.regions()
            .into_iter()
            .flat_map(move |r| {
                r.rows()
                    .map(move |row| RegionRow::new(r, row, advice_io, instance_io, fqr))
            })
            .collect()
    }

    /// Returns the certesian product between the regions' rows and the lookups
    pub fn lookups_per_region_row<'a, 'io, 'fq, F: Field>(
        &'a self,
        lookups: &[Lookup<'a, F>],
        advice_io: &'io AdviceIO,
        instance_io: &'io InstanceIO,
        fqr: &'fq dyn FixedQueryResolver<F>,
    ) -> Vec<(RegionRow<'a, 'io, 'fq, F>, Lookup<'a, F>)> {
        self.region_rows(advice_io, instance_io, fqr)
            .into_iter()
            .flat_map(|r| lookups.iter().copied().map(move |l| (r, l)))
            .collect()
    }

    pub fn is_top_level(&self) -> bool {
        matches!(self.kind, GroupKind::TopLevel)
    }

    pub fn inputs(&self) -> &[GroupCell] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[GroupCell] {
        &self.outputs
    }

    /// If the group has a children with that index return Some with the position in the children
    /// array where it is. Otherwise returns None
    pub fn has_child(&self, n: usize) -> Option<usize> {
        self.children.iter().position(|c| *c == n)
    }

    pub fn name(&self) -> &str {
        if self.kind == GroupKind::TopLevel {
            return "Main";
        }
        self.name
            .as_deref()
            .map(|s| if s.is_empty() { "unnamed_group" } else { s })
            .unwrap_or("unnamed_group")
    }

    pub fn children_count(&self) -> usize {
        self.children.len()
    }

    /// Returns the group objects of the children
    pub fn children<'a>(&'a self, groups: &'a [Group]) -> Vec<(usize, &'a Group)> {
        self.children
            .iter()
            .copied()
            .map(|idx| (idx, groups.get(idx).unwrap()))
            .collect()
    }

    /// Returns the group key
    pub fn key(&self) -> Option<GroupKeyInstance> {
        match self.kind {
            GroupKind::TopLevel => None,
            GroupKind::Group(group_key_instance) => Some(group_key_instance),
        }
    }

    /// Returns the cartesian product of the regions and the gates.
    pub fn region_gates<'a, F: Field>(
        &'a self,
        gates: &'a [&dyn GateAdaptor<F>],
    ) -> impl Iterator<Item = (&'a dyn GateAdaptor<F>, RegionData<'a>)> + 'a {
        self.regions()
            .into_iter()
            .flat_map(|r| gates.iter().map(move |g| (*g, r)))
    }
}

/// A collection of groups.
///
/// It is represented with a newtype to be able to add methods to this type.
#[derive(Debug)]
pub struct Groups(Vec<Group>);

impl Groups {
    pub fn top_level(&self) -> Option<&Group> {
        // When constructing the flattened version the top level
        // group will be the last one so we reverse the iterator to try find it
        // faster.
        self.0.iter().rev().find(|g| g.kind == GroupKind::TopLevel)
    }

    pub fn region_starts(&self) -> HashMap<RegionIndex, usize> {
        self.0
            .iter()
            .flat_map(|g| g.regions())
            .map(|r| {
                let idx = r
                    .index()
                    .unwrap_or_else(|| panic!("Region {r:?} does not have an index"));

                (idx, r.start().unwrap_or_default())
            })
            .collect()
    }
}

impl AsRef<[Group]> for Groups {
    fn as_ref(&self) -> &[Group] {
        self.0.as_ref()
    }
}

impl Borrow<[Group]> for Groups {
    fn borrow(&self) -> &[Group] {
        self.as_ref()
    }
}

impl Deref for Groups {
    type Target = Vec<Group>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represents a piece of the circuit's constraint system.
///
/// Is meant to be used for constructing the call graph. For
/// lowering use [`Groups`] instead.
/// Has a set of regions of the circuit that represents what gates
/// are enabled in it.
///
/// Can have children blocks that represent subpieces of the logic.
/// The boundary between parent and children is determined by the groups
/// the circuit declares during synthesis.
#[derive(Debug)]
pub struct GroupTree {
    kind: GroupKind,
    name: Option<String>,
    inputs: Vec<GroupCell>,
    outputs: Vec<GroupCell>,
    regions: Regions,
    children: Vec<GroupTree>,
}

impl GroupTree {
    /// Constructs an empty top-level group.
    fn top_level() -> Self {
        Self {
            kind: GroupKind::TopLevel,
            name: None,
            inputs: Default::default(),
            outputs: Default::default(),
            regions: Default::default(),
            children: Default::default(),
        }
    }

    /// Constructs an empty group
    fn new(name: String, key: impl GroupKey) -> Self {
        Self {
            kind: GroupKind::Group(key.into()),
            name: Some(name),
            inputs: Default::default(),
            outputs: Default::default(),
            regions: Default::default(),
            children: Default::default(),
        }
    }

    fn flatten_impl(self, groups: &mut Vec<Group>) {
        let mut child_indices = vec![];
        for child in self.children {
            child.flatten_impl(groups);
            child_indices.push(groups.len() - 1);
        }
        groups.push(Group::new(
            self.kind,
            self.name,
            self.inputs,
            self.outputs,
            self.regions,
            child_indices,
        ));
    }

    /// Transforms the tree into a read-only flat representation.
    pub fn flatten(self) -> Groups {
        let mut groups = vec![];
        self.flatten_impl(&mut groups);
        Groups(groups)
    }

    /// Returns the name of the group.
    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("<no name>")
    }
}

/// Manages the creation of groups during synthesis.
///
/// Starts with a top level group and automatically handles parent-children relations during
/// construction. New children are pushed into a stack until they are completely built.
/// Once completed they get added to the list of children of the next group in the stack.
///
/// The root group owned by the builder is always a top-level block.
#[derive(Debug)]
pub struct GroupBuilder {
    root: GroupTree,
    stack: Vec<GroupTree>,
}

impl GroupBuilder {
    /// Creates a builder with a top-level group as the root block.
    pub fn new() -> Self {
        Self {
            root: GroupTree::top_level(),
            stack: vec![],
        }
    }

    /// Returns a mutable reference to the group that is currently being built.
    /// Private to ensure that only the builder can mutate the groups.
    #[inline]
    pub fn current(&self) -> &GroupTree {
        if self.stack.is_empty() {
            &self.root
        } else {
            self.stack.last().unwrap()
        }
    }

    /// Returns a mutable reference to the group that is currently being built.
    /// Private to ensure that only the builder can mutate the groups.
    #[inline]
    fn current_mut(&mut self) -> &mut GroupTree {
        if self.stack.is_empty() {
            &mut self.root
        } else {
            self.stack.last_mut().unwrap()
        }
    }

    /// Returns the root and consumes the builder.
    ///
    /// Panics if there are pending groups on the stack.
    pub fn into_root(self) -> GroupTree {
        assert!(self.stack.is_empty(), "Builder has pending groups");
        self.root
    }

    /// Pushes a new group group into the stack.
    pub fn push<N, NR, K>(&mut self, name: N, key: K)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
        K: GroupKey,
    {
        self.stack.push(GroupTree::new(name().into(), key))
    }

    /// Pops the top of the stack and moves it to the list of children of the parent element.
    ///
    /// Panics if the stack is empty.
    pub fn pop(&mut self) {
        assert!(!self.stack.is_empty(), "No pending groups");
        let g = self.stack.pop().unwrap();
        self.current_mut().children.push(g);
    }

    /// Adds a cell to the current group's list of inputs.
    ///
    /// If the cell is a fixed cell it is ignored.
    pub fn add_input(&mut self, cell: impl Into<GroupCell>) {
        let cell = cell.into();
        if !cell.is_fixed() {
            self.current_mut().inputs.push(cell)
        }
    }

    /// Adds a cell to the current group's list of outputs.
    ///
    /// If the cell is a fixed cell it is ignored.
    pub fn add_output(&mut self, cell: impl Into<GroupCell>) {
        let cell = cell.into();
        if !cell.is_fixed() {
            self.current_mut().outputs.push(cell)
        }
    }

    /// Adds a cell to the root group's list of inputs.
    pub fn add_root_input<C: ColumnType>(&mut self, cell: IOCell<C>)
    where
        IOCell<C>: Into<GroupCell>,
    {
        self.current_mut().inputs.push(cell.into())
    }

    /// Adds a cell to the root group's list of outputs.
    pub fn add_root_output<C: ColumnType>(&mut self, cell: IOCell<C>)
    where
        IOCell<C>: Into<GroupCell>,
    {
        self.current_mut().outputs.push(cell.into())
    }

    /// Returns a mutable reference to the regions in the current group.
    pub fn regions_mut(&mut self) -> &mut Regions {
        &mut self.current_mut().regions
    }
}

impl Default for GroupBuilder {
    fn default() -> Self {
        Self::new()
    }
}
