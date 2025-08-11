mod constraint;
pub mod regions;

use std::{
    collections::{HashMap, HashSet},
    convert::identity,
};

use anyhow::{anyhow, Result};
use constraint::{EqConstraint, Graph};
use regions::{RegionData, RegionRow, Regions, TableData, FQN};

use crate::{
    gates::{find_gate_selector_set, AnyQuery},
    halo2::{Field, *},
    io::{AdviceIOValidator, InstanceIOValidator},
    lookups::{Lookup, LookupKind, LookupTableRow},
    value::steal,
    CircuitCallbacks, CircuitIO,
};

pub struct CircuitSynthesis<F: Field> {
    cs: ConstraintSystem<F>,
    regions: Regions<F>,
    #[cfg(feature = "phase-tracking")]
    current_phase: sealed::Phase,

    eq_constraints: Graph<EqConstraint>,
    materialized_fixed_cells: Vec<(usize, usize)>,
    out_of_region_fixed_cells: HashMap<(Column<Fixed>, usize), Value<F>>,
    advice_io: CircuitIO<Advice>,
    instance_io: CircuitIO<Instance>,
}

pub type AnyCell = (Column<Any>, usize);

impl<F: Field> CircuitSynthesis<F> {
    fn init<C: Circuit<F>, CB: CircuitCallbacks<F, C>>() -> (Self, C::Config) {
        let mut cs = ConstraintSystem::default();
        let config = C::configure(&mut cs);

        (
            Self {
                cs,
                regions: Default::default(),
                #[cfg(feature = "phase-tracking")]
                current_phase: FirstPhase.to_sealed(),
                advice_io: CB::advice_io(&config),
                instance_io: CB::instance_io(&config),
                eq_constraints: Default::default(),
                materialized_fixed_cells: Default::default(),
                out_of_region_fixed_cells: Default::default(),
            },
            config,
        )
    }

    pub fn new<C: Circuit<F>, CB: CircuitCallbacks<F, C>>(circuit: &C) -> Result<Self> {
        let (mut syn, config) = Self::init::<C, CB>();

        syn.synthesize(circuit, config)?;
        log::debug!("cs = {:?}", syn.cs);

        syn.advice_io.validate(&AdviceIOValidator)?;
        syn.instance_io
            .validate(&InstanceIOValidator::new(&syn.cs))?;
        Ok(syn)
    }

    pub fn gates(&self) -> &[Gate<F>] {
        self.cs.gates()
    }

    pub fn cs(&self) -> &ConstraintSystem<F> {
        &self.cs
    }

    pub fn regions<'a>(&'a self) -> Vec<RegionData<'a, F>> {
        self.regions.regions()
    }

    pub fn region_rows<'a>(&'a self) -> impl Iterator<Item = RegionRow<'a, 'a, F>> + 'a {
        self.regions().into_iter().flat_map(move |r| {
            r.rows()
                .map(move |row| RegionRow::new(r, row, self.advice_io(), self.instance_io()))
        })
    }

    pub fn lookups<'a>(&'a self) -> impl Iterator<Item = Lookup<'a, F>> + 'a {
        Lookup::load(self).into_iter()
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

    pub fn lookups_per_region_row<'a>(
        &'a self,
    ) -> impl Iterator<Item = (RegionRow<'a, 'a, F>, Lookup<'a, F>)> + 'a {
        self.region_rows()
            .flat_map(|r| self.lookups().map(move |l| (r, l)))
    }

    pub fn tables(&self) -> &[TableData<F>] {
        self.regions.tables()
    }

    fn find_table(&self, q: &[AnyQuery]) -> Result<Vec<Vec<F>>> {
        self.tables()
            .iter()
            .find_map(|table| table.get_rows(&q))
            .ok_or_else(|| anyhow!("Could not get values from table"))
            .and_then(identity)
    }

    fn tables_for_queries(&self, q: &[AnyQuery]) -> Result<Vec<LookupTableRow<F>>> {
        // For each table region look if they have the columns we are looking for and
        // collect all the fixed values
        let columns = q.iter().map(|q| q.column_index()).collect::<Vec<_>>();
        let table = self.find_table(q)?;
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
    }

    pub fn tables_for_lookup(&self, l: &Lookup<F>) -> Result<Vec<LookupTableRow<F>>> {
        l.table_queries().and_then(|q| self.tables_for_queries(&q))
    }

    pub fn regions_by_index(&self) -> HashMap<RegionIndex, RegionStart> {
        self.regions
            .regions()
            .into_iter()
            .enumerate()
            .map(|(idx, region)| (idx.into(), region.rows().start.into()))
            .collect()
    }

    pub fn advice_io(&self) -> &CircuitIO<Advice> {
        &self.advice_io
    }

    pub fn instance_io(&self) -> &CircuitIO<Instance> {
        &self.instance_io
    }

    pub fn constraints(&self) -> impl Iterator<Item = (AnyCell, AnyCell)> {
        self.eq_constraints.into_iter().copied()
    }

    pub fn sorted_constraints(&self) -> Vec<(AnyCell, AnyCell)> {
        let mut constraints = self.eq_constraints.iter().copied().collect::<Vec<_>>();
        constraints.sort();
        constraints
    }

    /// Returns an iterator with equality constraints
    pub fn fixed_constraints(&self) -> impl Iterator<Item = Result<(Column<Fixed>, usize, F)>> {
        let regions = self.regions();

        self.fixed_cells_in_eq_constraints()
            .flat_map(move |(col, row)| {
                let values = regions
                    .iter()
                    .enumerate()
                    .inspect(|(idx, r)| {
                        log::debug!(
                            "Cell ({}, {row}) | Looking in region {} '{}' ({}, {})",
                            col.index(),
                            idx,
                            r.name(),
                            r.rows().start,
                            r.rows().end
                        )
                    })
                    .filter_map(|(_, r)| {
                        // Try find a value assigned to the fixed column in this region
                        r.find_fixed_col_assignment(col, row)
                    })
                    .inspect(|v| log::debug!("Cell ({}, {row}) | Found {v:?}", col.index()))
                    .collect::<Vec<_>>();
                // The value can be missing but we don't support more than one assignment.
                assert!(values.len() <= 1);
                values.first().copied().map(|v| {
                    let f =
                        steal(&v).ok_or_else(|| anyhow!("Unknown value assigned to fixed cell"))?;
                    Ok((col, row, f))
                })
            })
    }

    fn fixed_cells_in_eq_constraints(&self) -> impl Iterator<Item = (Column<Fixed>, usize)> {
        self.eq_constraints
            .iter()
            .flat_map(|(l, r)| [l, r])
            .inspect(|c| log::debug!("Cell used in eq constraint: {c:?}"))
            .filter_map(|(c, r)| {
                let fc: Result<Column<Fixed>, _> = (*c).try_into();
                fc.ok().map(|fc| (fc, *r))
            })
            .inspect(|c| log::debug!("Fixed cell used in eq constraint: {c:?}"))
            .collect::<HashSet<_>>()
            .into_iter()
    }

    pub fn regions_ref(&self) -> &Regions<F> {
        &self.regions
    }

    pub fn region_gates<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a Gate<F>, RegionRow<'a, 'a, F>)> + 'a {
        self.regions()
            .into_iter()
            .map(|r| r.rows())
            .reduce(|lhs, rhs| std::cmp::min(lhs.start, rhs.start)..std::cmp::max(lhs.end, rhs.end))
            .unwrap_or(0..0)
            .flat_map(move |row| {
                self.regions().into_iter().filter_map(move |region| {
                    if region.rows().contains(&row) {
                        Some(RegionRow::new(
                            region,
                            row,
                            self.advice_io(),
                            self.instance_io(),
                        ))
                    } else {
                        None
                    }
                })
            })
            .flat_map(|r| {
                self.gates().iter().filter_map(move |gate| {
                    let selectors = find_gate_selector_set(gate.polynomials());
                    if r.gate_is_disabled(&selectors) {
                        return None;
                    }
                    Some((gate, r))
                })
            })
    }

    pub fn seen_advice_cells(&self) -> impl Iterator<Item = (&(usize, usize), &FQN)> {
        self.regions.seen_advice_cells()
    }

    fn maybe_materialize_fixed(&mut self, col: Column<Any>, row: usize) {
        if *col.column_type() == Any::Fixed {
            self.materialized_fixed_cells.push((col.index(), row));
        }
    }
}

#[cfg(not(feature = "phase-tracking"))]
impl<F: Field> CircuitSynthesis<F> {
    fn synthesize<C: Circuit<F>>(&mut self, circuit: &C, config: C::Config) -> Result<()> {
        let constants = self.cs.constants().clone();
        C::FloorPlanner::synthesize(self, circuit, config, constants)?;

        Ok(())
    }

    fn in_phase<P: Phase>(&self, _phase: P) -> bool {
        true
    }
}

#[cfg(feature = "phase-tracking")]
impl<F: Field> CircuitSynthesis<F> {
    fn synthesize<C: Circuit<F>>(&mut self, circuit: &C, config: C::Config) -> Result<()> {
        for current_phase in self.cs.phases() {
            self.current_phase = current_phase;

            C::FloorPlanner::synthesize(self, circuit, config.clone(), self.cs.constants.clone())?;
        }
        Ok(())
    }

    fn in_phase<P: Phase>(&self, phase: P) -> bool {
        self.current_phase == phase.to_sealed()
    }
}

impl<F: Field> Assignment<F> for CircuitSynthesis<F> {
    fn enter_region<NR, N>(&mut self, region_name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        if self.in_phase(FirstPhase) {
            self.regions.push(region_name);
        }
    }

    fn exit_region(&mut self) {
        if self.in_phase(FirstPhase) {
            self.regions.commit();
        }
    }

    fn enable_selector<A, AR>(&mut self, _: A, selector: &Selector, row: usize) -> Result<(), Error>
    where
        AR: Into<String>,
        A: FnOnce() -> AR,
    {
        self.regions.edit(|region| {
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
        self.regions.edit(|region| {
            region.update_extent(advice.into(), row);
            region.note_advice(advice, row, name().into());
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
        self.regions.edit_current_or_last(|region| {
            region.update_extent(fixed.into(), row);
            region.assign_fixed(fixed, row, value());
        });
        Ok(())
    }

    fn copy(
        &mut self,
        from: Column<Any>,
        from_row: usize,
        to: Column<Any>,
        to_row: usize,
    ) -> Result<(), Error> {
        self.maybe_materialize_fixed(from, from_row);
        self.maybe_materialize_fixed(to, to_row);
        self.eq_constraints.add((from, from_row, to, to_row));
        Ok(())
    }

    fn fill_from_row(
        &mut self,
        column: Column<Fixed>,
        row: usize,
        value: Value<Assigned<F>>,
    ) -> Result<(), Error> {
        log::debug!("fill_from_row{:?}", (column, row, value));
        self.regions.mark_current_as_table();
        self.regions
            .edit(|region| region.blanket_fill(column, row, value.map(|f| f.evaluate())));
        Ok(())
    }

    fn push_namespace<NR, N>(&mut self, name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.regions.edit(|region| region.push_namespace(name));
    }

    fn pop_namespace(&mut self, name: Option<String>) {
        self.regions.edit(|region| region.pop_namespace(name));
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
}

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
