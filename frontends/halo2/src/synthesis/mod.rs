//! Defines types for handling the result of synthesizing a circuit.

use std::{
    collections::{HashMap, HashSet},
    convert::identity,
};

use anyhow::{anyhow, Result};
use constraint::{EqConstraint, EqConstraintArg, EqConstraintGraph};
use groups::{Group, GroupBuilder, GroupCell, Groups};
use regions::{
    FixedData, RegionIndexToStart, TableData,
    FQN,
};

use crate::{
    gates::AnyQuery,
    halo2::{
        groups::{GroupKey, RegionsGroup},
        Field, *,
    },
    io::IOCell,
    lookups::{Lookup, LookupKind, LookupTableRow},
    value::steal,
    CircuitCallbacks, CircuitIO,
};

pub mod constraint;
pub mod groups;
pub mod regions;

pub type AnyCell = (Column<Any>, usize);

/// Result of synthesizing a circuit.
///
/// TODO: Cleanup this struct
pub struct CircuitSynthesis<F: Field> {
    cs: ConstraintSystem<F>,
    eq_constraints: EqConstraintGraph<F>,
    fixed: FixedData<F>,
    tables: Vec<TableData<F>>,
    advice_names: HashMap<(usize, usize), FQN>,
    groups: Groups,
}

impl<F: Field> CircuitSynthesis<F> {
    /// Returns the list of gates in the constraint system.
    pub fn gates(&self) -> &[Gate<F>] {
        self.cs.gates()
    }

    pub fn cs(&self) -> &ConstraintSystem<F> {
        &self.cs
    }

    pub fn lookups<'a>(&'a self) -> impl Iterator<Item = Lookup<'a, F>> + 'a {
        Lookup::load(&self.cs).into_iter()
    }

    pub fn lookup_kinds<'a>(&'a self) -> Result<HashMap<LookupKind, Vec<Lookup<'a, F>>>> {
        fn fold<'a, F: Field>(
            mut map: HashMap<LookupKind, Vec<Lookup<'a, F>>>,
            lookup: Result<(LookupKind, Lookup<'a, F>)>,
        ) -> Result<HashMap<LookupKind, Vec<Lookup<'a, F>>>> {
            lookup.map(|(k, l)| {
                map.entry(k).or_default().push(l);
                map
            })
        }

        self.lookups()
            .map(|l| Ok((l.kind()?, l)))
            .try_fold(HashMap::default(), fold)
    }

    fn find_table(&self, q: &[AnyQuery]) -> Result<Vec<Vec<F>>> {
        self.tables
            .iter()
            .find_map(|table| table.get_rows(&q))
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

            Ok(transpose(table)
                .into_iter()
                .map(|row| LookupTableRow::new(&columns, row))
                .collect())
        })
    }

    pub fn advice_io(&self) -> &CircuitIO<Advice> {
        self.groups
            .top_level()
            .expect("top level is missing")
            .advice_io()
    }

    pub fn instance_io(&self) -> &CircuitIO<Instance> {
        self.groups
            .top_level()
            .expect("top level is missing")
            .instance_io()
    }

    pub fn groups(&self) -> &[Group] {
        self.groups.as_ref()
    }

    pub fn constraints(&self) -> &EqConstraintGraph<F> {
        &self.eq_constraints
    }

    /// Returns a mapping from the region index to region start
    pub fn regions_by_index(&self) -> RegionIndexToStart {
        self.groups
            .as_ref()
            .iter()
            .flat_map(|g| g.regions())
            .into_iter()
            .enumerate()
            .map(|(idx, region)| (idx.into(), region.rows().start.into()))
            .collect()
    }

    /// Returns the top level group in the circuit.
    pub fn top_level_group(&self) -> Option<&Group> {
        self.groups.top_level()
    }

    /// Returns a sorted vector with copy constraints.
    //pub fn sorted_constraints(&self) -> Vec<EqConstraint> {
    //    let mut constraints = self.eq_constraints.iter().copied().collect::<Vec<_>>();
    //    constraints.sort();
    //    constraints
    //}

    ///// Returns an iterator with equality constraints
    //pub fn fixed_constraints(&self) -> impl Iterator<Item = Result<(Column<Fixed>, usize, F)>> {
    //    let regions = self.regions();
    //
    //    self.fixed_cells_in_eq_constraints()
    //        .flat_map(move |(col, row)| {
    //            let values = regions
    //                .iter()
    //                .enumerate()
    //                .inspect(|(idx, r)| {
    //                    log::debug!(
    //                        "Cell ({}, {row}) | Looking in region {} '{}' ({}, {})",
    //                        col.index(),
    //                        idx,
    //                        r.name(),
    //                        r.rows().start,
    //                        r.rows().end
    //                    )
    //                })
    //                .filter_map(|(_, r)| {
    //                    // Try find a value assigned to the fixed column in this region
    //                    r.find_fixed_col_assignment(col, row)
    //                })
    //                .inspect(|v| log::debug!("Cell ({}, {row}) | Found {v:?}", col.index()))
    //                .collect::<Vec<_>>();
    //            // The value can be missing but we don't support more than one assignment.
    //            assert!(values.len() <= 1);
    //            values.first().copied().map(|v| {
    //                let f =
    //                    steal(&v).ok_or_else(|| anyhow!("Unknown value assigned to fixed cell"))?;
    //                Ok((col, row, f))
    //            })
    //        })
    //}
    //
    //fn fixed_cells_in_eq_constraints(&self) -> impl Iterator<Item = (Column<Fixed>, usize)> {
    //    self.eq_constraints
    //        .iter()
    //        .flat_map(|(l, r)| [l, r])
    //        .inspect(|c| log::debug!("Cell used in eq constraint: {c:?}"))
    //        .filter_map(|(c, r)| {
    //            let fc: Result<Column<Fixed>, _> = (*c).try_into();
    //            fc.ok().map(|fc| (fc, *r))
    //        })
    //        .inspect(|c| log::debug!("Fixed cell used in eq constraint: {c:?}"))
    //        .collect::<HashSet<_>>()
    //        .into_iter()
    //}

    pub fn seen_advice_cells(&self) -> impl Iterator<Item = (&(usize, usize), &FQN)> {
        self.advice_names.iter()
    }
}

/// Collects the information from the synthesis.
#[derive(Default)]
pub(crate) struct Synthesizer<F: Field> {
    cs: ConstraintSystem<F>,
}

impl<F: Field> Synthesizer<F> {
    pub fn new() -> Self {
        Self {
            cs: Default::default(),
        }
    }

    pub fn cs(&self) -> &ConstraintSystem<F> {
        &self.cs
    }

    pub fn cs_mut(&mut self) -> &mut ConstraintSystem<F> {
        &mut self.cs
    }

    /// Synthetizes the given circuit and returns the collected information.
    ///
    /// This method consumes the synthetizer.
    pub fn synthesize<C: Circuit<F>>(
        self,
        circuit: &C,
        config: C::Config,
        advice_io: CircuitIO<Advice>,
        instance_io: CircuitIO<Instance>,
    ) -> Result<CircuitSynthesis<F>> {
        let mut eq_constraints = Default::default();
        // A list of set of columns. Represents the regions that need to be converted into tables.
        let mut tables: Vec<HashSet<Column<Fixed>>> = vec![];
        let mut fixed = FixedData::default();
        let mut advice_names = HashMap::<(usize, usize), FQN>::new();
        let mut region_indices = (0..).map(RegionIndex::from);
        let groups = {
            let mut inner = SynthesizerInner {
                eq_constraints: &mut eq_constraints,
                tables: &mut tables,
                fixed: &mut fixed,
                advice_names: &mut advice_names,
                next_index: &mut region_indices,
                groups: GroupBuilder::new(),
                #[cfg(feature = "phase-tracking")]
                current_phase: FirstPhase.to_sealed(),
            };
            add_root_io(&mut inner.groups, &advice_io);
            add_root_io(&mut inner.groups, &instance_io);

            inner.synthesize_inner(circuit, config, &self.cs)?;

            inner.groups.into_root().flatten()
        };

        let fixed = fixed;
        add_fixed_to_const_constraints(&mut eq_constraints, &fixed)?;
        let tables = fill_tables(tables, &fixed)?;

        Ok(CircuitSynthesis {
            cs: self.cs,
            eq_constraints,
            fixed,
            advice_names,
            tables,
            groups,
        })
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

/// Implementation of Assignment that records the information required to create the circuit
/// synthesis.
struct SynthesizerInner<'a, F: Field> {
    eq_constraints: &'a mut EqConstraintGraph<F>,
    tables: &'a mut Vec<HashSet<Column<Fixed>>>,
    fixed: &'a mut FixedData<F>,
    advice_names: &'a mut HashMap<(usize, usize), FQN>,
    next_index: &'a mut dyn Iterator<Item = RegionIndex>,
    groups: GroupBuilder,
    #[cfg(feature = "phase-tracking")]
    current_phase: sealed::Phase,
}

#[cfg(not(feature = "phase-tracking"))]
impl<F: Field> SynthesizerInner<'_, F> {
    /// Inner method that calls the floor planner's synthesize method.
    /// This method is separated from the rest of the synthetization
    /// method because its logic depends on the phase-tracking feature being enabled or not.
    fn synthesize_inner<C: Circuit<F>>(
        &mut self,
        circuit: &C,
        config: C::Config,
        cs: &ConstraintSystem<F>,
    ) -> Result<()> {
        let constants = cs.constants().clone();
        C::FloorPlanner::synthesize(self, circuit, config, constants)?;

        Ok(())
    }

    fn in_phase<P: Phase>(&self, _phase: P) -> bool {
        true
    }
}

#[cfg(feature = "phase-tracking")]
impl<F: Field> SynthesizerInner<F> {
    /// Inner method that calls the floor planner's synthesize method.
    /// This method is separated from the rest of the synthetization
    /// method because its logic depends on the phase-tracking feature being enabled or not.
    fn synthesize_inner<C: Circuit<F>>(
        &mut self,
        circuit: &C,
        config: C::Config,
        cs: &ConstraintSystem<F>,
    ) -> Result<()> {
        for current_phase in self.cs.phases() {
            self.current_phase = current_phase;

            C::FloorPlanner::synthesize(self, circuit, config.clone(), cs.constants.clone())?;
        }
    }

    fn in_phase<P: Phase>(&self, phase: P) -> bool {
        self.current_phase == phase
    }
}

impl<F: Field> Assignment<F> for SynthesizerInner<'_, F> {
    fn enter_region<NR, N>(&mut self, region_name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        if self.in_phase(FirstPhase) {
            self.groups.regions_mut().push(region_name, self.next_index);
        }
    }

    fn exit_region(&mut self) {
        if self.in_phase(FirstPhase) {
            self.groups.regions_mut().commit();
        }
    }

    fn enable_selector<A, AR>(&mut self, _: A, selector: &Selector, row: usize) -> Result<(), Error>
    where
        AR: Into<String>,
        A: FnOnce() -> AR,
    {
        self.groups.regions_mut().edit(|region| {
            region.enable_selector(*selector, row);
        });
        Ok(())
    }

    fn query_instance(&self, _column: Column<Instance>, _row: usize) -> Result<Value<F>, Error> {
        Ok(Value::unknown())
    }

    fn assign_advice<V, VR, A, AR>(
        &mut self,
        name: A,
        advice: Column<Advice>,
        row: usize,
        _value: V,
    ) -> Result<(), Error>
    where
        VR: Into<Assigned<F>>,
        AR: Into<String>,
        V: FnOnce() -> Value<VR>,
        A: FnOnce() -> AR,
    {
        self.groups.regions_mut().edit(|region| {
            region.update_extent(advice.into(), row);
            let fqn = FQN::new(
                region.name(),
                region.index(),
                //&self.inner.namespaces,
                &[],
                Some(name().into()),
            );
            log::debug!(
                "Recording advice assignment @ col = {}, row = {row}, name = {fqn}",
                advice.index()
            );
            self.advice_names.insert((advice.index(), row), fqn);
        });
        Ok(())
    }

    fn assign_fixed<V, VR, A, AR>(
        &mut self,
        _: A,
        fixed: Column<Fixed>,
        row: usize,
        value: V,
    ) -> Result<(), Error>
    where
        VR: Into<Assigned<F>>,
        AR: Into<String>,
        V: FnOnce() -> Value<VR>,
        A: FnOnce() -> AR,
    {
        // Assignments to fixed cells can happen outside a region so we write those on the last
        // region if available
        self.groups.regions_mut().edit(|region| {
            region.update_extent(fixed.into(), row);
        });
        self.fixed.assign_fixed(fixed, row, value());
        Ok(())
    }

    fn copy(
        &mut self,
        from: Column<Any>,
        from_row: usize,
        to: Column<Any>,
        to_row: usize,
    ) -> Result<(), Error> {
        self.eq_constraints
            .add(EqConstraint::AnyToAny(from, from_row, to, to_row));
        Ok(())
    }

    fn fill_from_row(
        &mut self,
        column: Column<Fixed>,
        row: usize,
        value: Value<Assigned<F>>,
    ) -> Result<(), Error> {
        log::debug!("fill_from_row{:?}", (column, row, value));
        self.fixed
            .blanket_fill(column, row, value.map(|f| f.evaluate()));
        let r = self.groups.regions_mut();
        r.edit(|region| region.update_extent(column.into(), row));
        r.move_latest_to_tables(self.tables);
        Ok(())
    }

    fn push_namespace<NR, N>(&mut self, name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.groups
            .regions_mut()
            .edit(|region| region.push_namespace(name));
    }

    fn pop_namespace(&mut self, name: Option<String>) {
        self.groups
            .regions_mut()
            .edit(|region| region.pop_namespace(name));
    }

    #[cfg(feature = "annotate-column")]
    fn annotate_column<A, AR>(&mut self, _: A, _: Column<Any>)
    where
        AR: Into<String>,
        A: FnOnce() -> AR,
    {
        todo!()
    }

    #[cfg(feature = "get-challenge")]
    fn get_challenge(&self, _: Challenge) -> Value<F> {
        todo!()
    }

    fn enter_group<NR, N, K>(&mut self, name_fn: N, key: K)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
        K: GroupKey,
    {
        self.groups.push(name_fn, key)
    }

    fn exit_group(&mut self, meta: RegionsGroup) {
        for input in meta.inputs() {
            self.groups.add_input(input);
        }
        for output in meta.outputs() {
            self.groups.add_output(output);
        }
        self.groups.pop();
    }
}
