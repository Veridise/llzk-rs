use crate::halo2::{Advice, Column, ColumnType, Instance};
use anyhow::{Result, bail};
use std::collections::HashSet;
use std::hash::Hash;

pub type IOCell<C> = (Column<C>, usize);

pub type AdviceIO = CircuitIO<Advice>;
pub type InstanceIO = CircuitIO<Instance>;

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

    /// Returns the cells that are inputs.
    pub fn inputs(&self) -> &[IOCell<C>] {
        &self.inputs
    }

    /// Returns the number of inputs.
    pub fn inputs_count(&self) -> usize {
        self.inputs.len()
    }

    /// Returns the cells that are outputs.
    pub fn outputs(&self) -> &[IOCell<C>] {
        &self.outputs
    }

    /// Returns the number of outputs.
    pub fn outputs_count(&self) -> usize {
        self.outputs.len()
    }

    fn map(m: &[(Column<C>, &[usize])]) -> Vec<IOCell<C>> {
        m.iter()
            .flat_map(|(col, rows)| rows.iter().map(|row| (*col, *row)))
            .collect()
    }
}

impl<C: ColumnType + Hash> CircuitIO<C> {
    /// Creates a CircuitIO with the given columns and each row that is either an input or an
    /// output.
    pub fn new(
        inputs: &[(Column<C>, &[usize])],
        outputs: &[(Column<C>, &[usize])],
    ) -> Result<Self> {
        Self::new_from_iocells(Self::map(inputs), Self::map(outputs)).validated()
    }

    /// Creates a CircuitIO with only inputs.
    pub fn from_inputs(inputs: &[(Column<C>, &[usize])]) -> Result<Self> {
        Self::new(inputs, &[])
    }

    /// Creates a CircuitIO with only outputs.
    pub fn from_outputs(outputs: &[(Column<C>, &[usize])]) -> Result<Self> {
        Self::new(&[], outputs)
    }

    fn validated(self) -> Result<Self> {
        let inputs = self.input_set();
        let outputs = self.output_set();

        if !inputs.is_disjoint(&outputs) {
            bail!("Sets are not disjoint");
        }
        Ok(self)
    }

    #[inline]
    fn input_set(&self) -> HashSet<&IOCell<C>> {
        self.inputs.iter().collect()
    }

    #[inline]
    fn output_set(&self) -> HashSet<&IOCell<C>> {
        self.outputs.iter().collect()
    }
}
