use std::{
    borrow::Borrow,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use ff::Field;

use crate::{
    io::{AdviceIO, CircuitIO, IOCell, InstanceIO},
    resolvers::FixedQueryResolver,
};
use halo2_frontend_core::{
    expressions::ExprBuilder,
    query::{Advice, Instance},
    table::{Any, Cell, ColumnType, RegionIndex, Rotation, RotationExt},
};

use super::regions::{RegionData, RegionRow, Regions};

pub type GroupKey = u64;

/// A group can either represent the circuit itself (the top level)
/// or a group declared during synthesis, identified by its key.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupKind {
    TopLevel,
    Group(GroupKey),
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
    pub fn to_expr<F: Field, E: ExprBuilder<F>>(self) -> E {
        match self {
            GroupCell::Assigned(cell) => cell.column.query_cell(Rotation::cur()),
            GroupCell::InstanceIO((column, _)) => column.query_cell(Rotation::cur()),
            GroupCell::AdviceIO((column, _)) => column.query_cell(Rotation::cur()),
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
            GroupCell::Assigned(cell) => Some(cell.region_index.into()),
            _ => None,
        }
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

/// Shared implementation of a group.
#[derive(Debug)]
pub(crate) struct GroupBase {
    kind: GroupKind,
    name: Option<String>,
    inputs: Vec<GroupCell>,
    outputs: Vec<GroupCell>,
    regions: Regions,
}

impl GroupBase {
    /// Constructs an empty top-level group.
    fn top_level() -> Self {
        Self {
            kind: GroupKind::TopLevel,
            name: None,
            inputs: Default::default(),
            outputs: Default::default(),
            regions: Default::default(),
        }
    }

    /// Constructs an named empty group.
    fn empty(name: String, key: GroupKey) -> Self {
        Self {
            kind: GroupKind::Group(key),
            name: Some(name),
            inputs: Default::default(),
            outputs: Default::default(),
            regions: Default::default(),
        }
    }

    /// Returns a list of region data.
    pub fn regions<'a>(&'a self) -> Vec<RegionData<'a>> {
        self.regions.regions()
    }

    pub fn name(&self) -> &str {
        if self.kind == GroupKind::TopLevel {
            return "Main";
        }
        self.name
            .as_deref()
            .and_then(|s| (!s.is_empty()).then_some(s))
            .unwrap_or("unnamed_group")
    }
}

/// A flat read-only representation of a group.
///
/// The parent-children relation is represented by indices on a vector instead.
#[derive(Debug)]
pub(crate) struct Group {
    base: GroupBase,
    children: Vec<usize>,
}

impl Group {
    fn with_children(base: GroupBase, children: Vec<usize>) -> Self {
        // Sanity check; if group is top level there cannot be any assigned cells.
        if base.kind == GroupKind::TopLevel {
            assert!(
                base.inputs
                    .iter()
                    .all(|i| !matches!(i, GroupCell::Assigned(_))),
                "Cannot assign input cells in the top level"
            );
            assert!(
                base.outputs
                    .iter()
                    .all(|i| !matches!(i, GroupCell::Assigned(_))),
                "Cannot assign output cells in the top level"
            );
        }
        Self { base, children }
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

    pub fn inputs(&self) -> &[GroupCell] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[GroupCell] {
        &self.outputs
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
    pub fn key(&self) -> Option<GroupKey> {
        match self.kind {
            GroupKind::TopLevel => None,
            GroupKind::Group(group_key_instance) => Some(group_key_instance),
        }
    }
}

impl Deref for Group {
    type Target = GroupBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Group {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// A collection of groups.
///
/// It is represented with a newtype to be able to add methods to this type.
#[derive(Debug)]
pub(crate) struct Groups(Vec<Group>);

impl Groups {
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
pub(crate) struct GroupTree {
    base: GroupBase,
    children: Vec<GroupTree>,
}

impl GroupTree {
    /// Constructs an empty top-level group.
    fn top_level() -> Self {
        Self {
            base: GroupBase::top_level(),
            children: Default::default(),
        }
    }

    /// Constructs an empty group
    fn new(name: String, key: GroupKey) -> Self {
        Self {
            base: GroupBase::empty(name, key),
            children: Default::default(),
        }
    }

    fn flatten_impl(self, groups: &mut Vec<Group>) {
        let mut child_indices = vec![];
        for child in self.children {
            child.flatten_impl(groups);
            child_indices.push(groups.len() - 1);
        }
        groups.push(Group::with_children(self.base, child_indices));
    }

    /// Transforms the tree into a read-only flat representation.
    pub fn flatten(self) -> Groups {
        let mut groups = vec![];
        self.flatten_impl(&mut groups);
        Groups(groups)
    }
}

impl Deref for GroupTree {
    type Target = GroupBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for GroupTree {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
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
    pub fn push(&mut self, name: String, key: GroupKey) {
        self.stack.push(GroupTree::new(name, key))
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

    /// Adds to the list of input and output cells of the top-level block.
    pub fn add_root_io<C: ColumnType>(&mut self, io: &CircuitIO<C>)
    where
        IOCell<C>: Into<GroupCell>,
    {
        self.root
            .inputs
            .extend(io.inputs().iter().copied().map(Into::into));
        self.root
            .outputs
            .extend(io.outputs().iter().copied().map(Into::into));
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
