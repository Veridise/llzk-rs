use std::{
    borrow::Cow,
    collections::VecDeque,
    hash::{DefaultHasher, Hasher as _},
    ops::RangeInclusive,
};

use helpers::RegionLayouterHelper;
use name::RegionName;
use trackers::RegionIndexTracker;

use crate::{
    error::to_plonk_error,
    halo2::{
        Advice, Any, Assigned, Cell, Challenge, Column, Error, Field, Fixed, Instance, Layouter,
        Region, RegionIndex, RegionLayouter, Selector, Table, Value,
    },
    io::CircuitIO,
};

use super::roles::{CellRole, Roles};

mod helpers;
mod name;
mod trackers;

/// Hints that a set of regions should be lowered as a separated module.
///
/// The backend does not
/// guarantee that it will honor this hint but it may rely on it for different reasons.
///
/// The hint requires annotations for advice cells that act as inputs and output of the region.
/// The backend may impose restrictions on these annotations. For instance, the Picus backend
/// requires that at least one cell is marked as output.
pub struct IsolatedRegions {
    io: CircuitIO<Advice>,
    tracker: Box<dyn RegionIndexTracker>,
    name: RegionName,
}

impl IsolatedRegions {
    fn new(tracker: impl RegionIndexTracker + 'static) -> Self {
        Self {
            io: Default::default(),
            tracker: Box::new(tracker),
            name: Default::default(),
        }
    }
}

/// This trait serves as an unique identifier of an isolation point. Two isolated region sets are
/// considered equal if they have the same isolation point. `IsolatedRegionsLayouter` uses this
/// point to group together different executions over the same chip's logic, which informs the
/// backend about potential callsites to the isolated module.
pub trait IsolationPoint: std::hash::Hash {}

/// A concrete isolation point key.
struct IsolationPointKey(u64);

impl<IP: IsolationPoint> From<IP> for IsolationPointKey {
    fn from(value: IP) -> Self {
        let mut h = DefaultHasher::new();
        value.hash(&mut h);
        Self(h.finish())
    }
}

/// Helper for constructing a set of isolated regions. Implements Layouter to allow using it as a drop-in replacement.
pub struct IsolatedRegionsLayouter<'a, L, IP> {
    inner: &'a mut L,
    regions: IsolatedRegions,
    point: IP,
    commited: bool,
}

impl<'a, L, IP> IsolatedRegionsLayouter<'a, L, IP> {
    pub fn new(inner: &'a mut L, tracker: impl RegionIndexTracker + 'static, point: IP) -> Self
    where
        IP: IsolationPoint,
    {
        Self {
            inner,
            regions: IsolatedRegions::new(tracker),
            point,
            commited: false,
        }
    }

    /// Annotates an advice cell with a certain role.
    ///
    /// If the cell is from one of the isolated regions it can have any role. The method will
    /// return an error if a cell is given an output role and is not from one of the isolated
    /// regions, even if it's the correct cell. For this reason it is recommended to annotate output cells after regions have been
    /// assigned.
    ///
    /// If the cell is from
    /// outside it can only have the input role and there has to be a transtive equality constraint
    /// between at least one cell in the region and the given cell. This requirement is checked
    /// during lowering since it cannot be checked here.
    ///
    /// If the conditions above are valid and the cell is going to be annotated, the method will
    /// return an error if the cell is not an advice cell.
    pub fn annotate_cell(&mut self, cell: Cell, role: CellRole<Advice>) -> Result<(), Error> {
        match *role {
            Roles::Input => self.regions.io.add(cell, role).map_err(Into::into),
            Roles::Output => {
                if !self.regions.tracker.contains(cell.region_index) {
                    return Err(to_plonk_error(format!(
                        "Cell with output role in region {} is not in the set of isolated regions",
                        *cell.region_index
                    )));
                }
                self.regions.io.add(cell, role).map_err(Into::into)
            }
            _ => Ok(()), // We don't care about roles other than IO
        }
    }
}

impl<F, L, IP> Layouter<F> for IsolatedRegionsLayouter<'_, L, IP>
where
    L: Layouter<F>,
    F: Field,
{
    type Root = L;

    /// Keeps track of the region and annotates it as isolated.
    fn assign_region<A, AR, N, NR>(&mut self, name: N, mut assignment: A) -> Result<AR, Error>
    where
        A: FnMut(Region<'_, F>) -> Result<AR, Error>,
        N: Fn() -> NR,
        NR: Into<String>,
    {
        self.regions.name.auto(&name);

        self.inner.assign_region(name, |inner_region| {
            let mut helper = RegionLayouterHelper::from(inner_region);
            let r = assignment(helper.wrapped())?;
            if let Some(index) = helper.index() {
                self.regions.tracker.update(index);
            }

            Ok(r)
        })
    }

    /// Tables are ignored and forwarded to the inner layouter.
    fn assign_table<A, N, NR>(&mut self, name: N, assignment: A) -> Result<(), Error>
    where
        A: FnMut(Table<'_, F>) -> Result<(), Error>,
        N: Fn() -> NR,
        NR: Into<String>,
    {
        self.inner.assign_table(name, assignment)
    }

    fn constrain_instance(
        &mut self,
        cell: Cell,
        column: Column<Instance>,
        row: usize,
    ) -> Result<(), Error> {
        self.inner.constrain_instance(cell, column, row)
    }

    fn get_challenge(&self, challenge: Challenge) -> Value<F> {
        self.inner.get_challenge(challenge)
    }

    fn get_root(&mut self) -> &mut Self::Root {
        &mut self.inner
    }

    fn push_namespace<NR, N>(&mut self, name_fn: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.inner.push_namespace(name_fn)
    }

    fn pop_namespace(&mut self, gadget_name: Option<String>) {
        self.pop_namespace(gadget_name)
    }
}

macro_rules! validate_regions {
    ($cond:expr, $msg:expr) => {
        debug_assert!($cond, "Isolated regions validation error: {}", $msg);
    };
}

impl<L, IP> Drop for IsolatedRegionsLayouter<'_, L, IP> {
    /// In debug builds validates that the regions are valid and have been commited. Failing to do so is an
    /// error.
    fn drop(&mut self) {
        validate_regions!(self.regions.name.has_value(), "Missing name");
        debug_assert!(self.commited, "Uncommited isolated regions");
    }
}
