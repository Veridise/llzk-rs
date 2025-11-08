use anyhow::{Result, bail};
use halo2_frontend_core::{
    query::{Advice, Instance},
    table::{Column, ColumnType},
};
use std::collections::HashSet;
use std::hash::Hash;

pub type IOCell<C> = (Column<C>, usize);

/// [`CircuitIO`] configured for [`Advice`] cells.
pub type AdviceIO = CircuitIO<Advice>;

/// [`CircuitIO`] configured for [`Instance`] cells.
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

    fn map<I>(m: &[(I, &[usize])]) -> Vec<IOCell<C>>
    where
        I: Into<Column<C>> + Copy,
    {
        m.iter()
            .flat_map(|(col, rows)| rows.iter().map(|row| ((*col).into(), *row)))
            .collect()
    }
}

impl<C: ColumnType + Hash> CircuitIO<C> {
    /// Creates a CircuitIO with the given columns and each row that is either an input or an
    /// output.
    pub fn new<I>(inputs: &[(I, &[usize])], outputs: &[(I, &[usize])]) -> Result<Self>
    where
        I: Into<Column<C>> + Copy,
    {
        Self::new_from_iocells(Self::map(inputs), Self::map(outputs)).validated()
    }

    /// Creates a CircuitIO with only inputs.
    pub fn from_inputs<I>(inputs: &[(I, &[usize])]) -> Result<Self>
    where
        I: Into<Column<C>> + Copy,
    {
        Self::new(inputs, &[])
    }

    /// Creates a CircuitIO with only outputs.
    pub fn from_outputs<I>(outputs: &[(I, &[usize])]) -> Result<Self>
    where
        I: Into<Column<C>> + Copy,
    {
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
