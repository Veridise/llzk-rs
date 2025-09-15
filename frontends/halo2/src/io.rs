use crate::halo2::{Advice, Column, ColumnType, ConstraintSystem, Field, Instance};
use anyhow::{bail, Result};
use std::borrow::Cow;
use std::collections::HashSet;
use std::hash::Hash;

pub type IOCell<C> = (Column<C>, usize);

#[derive(Debug, Clone)]
pub(crate) struct AllCircuitIO<'io> {
    pub advice: Cow<'io, CircuitIO<Advice>>,
    pub instance: Cow<'io, CircuitIO<Instance>>,
}

impl AllCircuitIO<'_> {
    pub fn input_count(&self) -> usize {
        self.instance.inputs().len() + self.advice.inputs().len()
    }

    pub fn output_count(&self) -> usize {
        self.instance.outputs().len() + self.advice.outputs().len()
    }
}

/// Records what cells of the given column type are inputs and what cells are outputs.
#[derive(Debug, Clone)]
pub struct CircuitIO<C: ColumnType> {
    inputs: Vec<IOCell<C>>,
    outputs: Vec<IOCell<C>>,
}

impl<C: ColumnType> CircuitIO<C> {
    /// Creates an empty CircuitIO without any inputs and outputs.
    pub fn empty() -> Self {
        Self {
            inputs: Default::default(),
            outputs: Default::default(),
        }
    }

    /// Creates a CircuitIO from a list of IOCells.
    pub(crate) fn new_from_iocells(
        inputs: impl IntoIterator<Item = IOCell<C>>,
        outputs: impl IntoIterator<Item = IOCell<C>>,
    ) -> Self {
        Self {
            inputs: Vec::from_iter(inputs),
            outputs: Vec::from_iter(outputs),
        }
    }

    pub(crate) fn add_input(&mut self, cell: IOCell<C>) {
        self.inputs.push(cell)
    }

    /// Creates a CircuitIO with the given columns and each row that is either an input or an
    /// output.
    pub fn new(inputs: &[(Column<C>, &[usize])], outputs: &[(Column<C>, &[usize])]) -> Self {
        Self::new_from_iocells(Self::map(inputs), Self::map(outputs))
    }

    /// Creates a CircuitIO with only inputs.
    pub fn from_inputs(inputs: &[(Column<C>, &[usize])]) -> Self {
        Self::new(inputs, &[])
    }

    /// Creates a CircuitIO with only outputs.
    pub fn from_outputs(outputs: &[(Column<C>, &[usize])]) -> Self {
        Self::new(&[], outputs)
    }

    pub fn inputs(&self) -> &[IOCell<C>] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[IOCell<C>] {
        &self.outputs
    }

    fn map(m: &[(Column<C>, &[usize])]) -> Vec<IOCell<C>> {
        m.iter()
            .flat_map(|(col, rows)| rows.iter().map(|row| (*col, *row)))
            .collect()
    }

    pub(crate) fn validate<V>(&self, validator: &V) -> Result<()>
    where
        V: IOValidator<C = C>,
    {
        validator.validate(self)
    }
}

type CellSet<C> = HashSet<IOCell<C>>;
type CellSetPair<C> = (CellSet<C>, CellSet<C>);

/// A validator for a particular column type.
pub(crate) trait IOValidator {
    type C: ColumnType + Hash;

    fn validate(&self, io: &CircuitIO<Self::C>) -> Result<()>;

    fn sets_are_disjoint(&self, io: &CircuitIO<Self::C>) -> Result<CellSetPair<Self::C>> {
        let inputs = self.input_set(io);
        let outputs = self.output_set(io);

        if !inputs.is_disjoint(&outputs) {
            bail!("Sets are not disjoint");
        }
        Ok((inputs, outputs))
    }

    #[inline]
    fn cell_set(&self, cells: &[IOCell<Self::C>]) -> HashSet<IOCell<Self::C>> {
        cells.iter().copied().collect()
    }

    #[inline]
    fn input_set(&self, io: &CircuitIO<Self::C>) -> HashSet<IOCell<Self::C>> {
        self.cell_set(&io.inputs)
    }

    #[inline]
    fn output_set(&self, io: &CircuitIO<Self::C>) -> HashSet<IOCell<Self::C>> {
        self.cell_set(&io.outputs)
    }
}

pub(crate) struct AdviceIOValidator;

impl IOValidator for AdviceIOValidator {
    type C = Advice;

    /// The advice IO specification is valid iff the set of inputs and outputs is disjoint.
    fn validate(&self, io: &CircuitIO<Self::C>) -> Result<()> {
        self.sets_are_disjoint(io).map(|_| {})
    }
}

pub(crate) struct InstanceIOValidator<'a, F: Field>(#[allow(dead_code)] &'a ConstraintSystem<F>);

impl<'a, F: Field> InstanceIOValidator<'a, F> {
    pub fn new(cs: &'a ConstraintSystem<F>) -> Self {
        Self(cs)
    }
}

impl<F: Field> IOValidator for InstanceIOValidator<'_, F> {
    type C = Instance;

    /// The instance IO specification is valid iff the set of inputs and outputs is disjoint and their
    /// union contains all the instance columns in the circuit.
    fn validate(&self, io: &CircuitIO<Self::C>) -> Result<()> {
        self.sets_are_disjoint(io).map(|_| {})
    }
}
