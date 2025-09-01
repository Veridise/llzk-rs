use crate::{
    error::to_plonk_error,
    halo2::{Advice, Any, Cell, Column, ColumnType, ConstraintSystem, Field, Instance},
    support::roles::{CellRole, Roles, SupportsInput, SupportsOutput},
};
use anyhow::{bail, Result};
use std::{collections::HashSet, hash::Hash};

/// Cell location.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct IOCell<C: ColumnType>(pub Column<C>, pub usize);

impl<C> TryFrom<Cell> for IOCell<C>
where
    C: ColumnType,
    Column<C>: TryFrom<Column<Any>, Error = &'static str>,
{
    type Error = IOError;

    fn try_from(cell: Cell) -> std::result::Result<Self, Self::Error> {
        Ok(IOCell(
            Column::<C>::try_from(cell.column).map_err(|e| IOError::ColumnConversionError(e))?,
            cell.row_offset,
        ))
    }
}

// List of cells with an associated role grouped by their column.
pub type IOCellList<'a, C> = [(Column<C>, &'a [(usize, CellRole<C>)])];

/// A cell of the circuit annotated with a role.
#[derive(Debug, Copy, Clone)]
struct CellWithRole<C: ColumnType>(Column<C>, usize, CellRole<C>);

impl<C: ColumnType> CellWithRole<C> {
    pub fn role(&self) -> Roles {
        *self.2
    }
}

impl<C: ColumnType> From<CellWithRole<C>> for IOCell<C> {
    fn from(value: CellWithRole<C>) -> Self {
        IOCell(value.0, value.1)
    }
}

/// Records hints of what cells of the given column type are inputs and what cells are outputs.
#[derive(Debug, Default)]
pub struct CircuitIO<C: ColumnType>(Vec<CellWithRole<C>>);

macro_rules! prep_rows {
    ($rows:expr, $C:ty, $out:ident, $sto:ident) => {{
        $sto = $rows
            .into_iter()
            .copied()
            .map(|(col, rows)| (col, CellRole::<$C>::inputs(rows.into_iter().copied())))
            .collect::<Vec<_>>();
        $out.extend($sto.iter().map(|(col, rows)| (*col, rows.as_slice())));
    }};
}

impl<C: SupportsInput + SupportsOutput> CircuitIO<C> {
    /// Creates a CircuitIO with the given columns and each row that is either an input or an
    /// output.
    ///
    /// Kept for compatibility.
    pub fn new(inputs: &[(Column<C>, &[usize])], outputs: &[(Column<C>, &[usize])]) -> Self {
        let mut roles = vec![];
        let mut isto = vec![];
        prep_rows!(inputs, C, roles, isto);
        let mut osto = vec![];
        prep_rows!(outputs, C, roles, osto);

        Self::new_with_roles(&roles)
    }
}

impl<C: ColumnType> CircuitIO<C> {
    /// Creates an empty CircuitIO without any inputs and outputs.
    pub fn empty() -> Self {
        Self(Default::default())
    }

    /// Creates a CircuitIO with the given columns and each row with a role.
    pub fn new_with_roles<'a>(cells: &IOCellList<'a, C>) -> Self {
        Self(Self::map(cells))
    }

    fn by_role(&self, role: Roles) -> Vec<IOCell<C>> {
        self.0
            .iter()
            .filter_map(|cell| {
                if cell.role() == role {
                    Some((*cell).into())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn add(
        &mut self,
        cell: impl TryInto<IOCell<C>, Error = IOError>,
        role: CellRole<C>,
    ) -> std::result::Result<(), IOError> {
        let cell: IOCell<C> = cell.try_into()?;
        self.0.push(CellWithRole(cell.0, cell.1, role));
        Ok(())
    }

    pub fn inputs(&self) -> Vec<IOCell<C>> {
        self.by_role(Roles::Input)
    }

    pub fn outputs(&self) -> Vec<IOCell<C>> {
        self.by_role(Roles::Output)
    }

    fn map<'a>(m: &IOCellList<'a, C>) -> Vec<CellWithRole<C>> {
        m.iter()
            .copied()
            .flat_map(|(col, rows)| {
                rows.iter()
                    .copied()
                    .map(move |(row, role)| CellWithRole(col, row, role))
            })
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
        self.cell_set(&io.inputs())
    }

    #[inline]
    fn output_set(&self, io: &CircuitIO<Self::C>) -> HashSet<IOCell<Self::C>> {
        self.cell_set(&io.outputs())
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

#[derive(Debug)]
pub enum IOError {
    ColumnConversionError(&'static str),
}

impl std::error::Error for IOError {}

impl std::fmt::Display for IOError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IOError::ColumnConversionError(s) => {
                write!(f, "Type conversion error between columns: {s}")
            }
        }
    }
}

impl From<IOError> for crate::halo2::Error {
    fn from(value: IOError) -> Self {
        to_plonk_error(value)
    }
}
