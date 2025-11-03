//! Adaptor traits that clients need to implement for communicating with the [`crate::driver::Driver`].

use ff::Field;

use crate::{QueryKind, halo2::Rotation, lookups::LookupData, table::Cell};

/// Boxed constraints system adaptor.
pub(crate) type CSI<F> = Box<dyn ConstraintSystemInfo<F>>;

/// Trait for querying information about the constraint system derived during configuration.
pub trait ConstraintSystemInfo<F: Field> {
    /// Returns the list of gates defined in the system.
    fn gates(&self) -> Vec<&dyn GateInfo<F>>;

    /// Returns a list with data about the lookups defined in the system.
    fn lookups<'cs>(&'cs self) -> Vec<LookupData<'cs, F>>;
}

/// Temporary implementation of [`ConstraintSystemAdaptor`].
impl<F: Field> ConstraintSystemInfo<F> for halo2_proofs::plonk::ConstraintSystem<F> {
    fn gates(&self) -> Vec<&dyn GateInfo<F>> {
        self.gates().iter().map(|g| g as &dyn GateInfo<F>).collect()
    }

    fn lookups<'cs>(&'cs self) -> Vec<LookupData<'cs, F>> {
        self.lookups()
            .iter()
            .map(|a| LookupData {
                name: a.name(),
                arguments: a.input_expressions(),
                table: a.table_expressions(),
            })
            .collect()
    }
}

/// Trait for querying information about the a gate in the constraint system.
pub trait GateInfo<F> {
    /// Returns the name of the gate.
    fn name(&self) -> &str;

    /// Returns the list of polynomials that make up the gate.
    fn polynomials(&self) -> &[crate::halo2::Expression<F>];
}

/// Temporary implementation of [`GateAdaptor`].
impl<F: Field> GateInfo<F> for halo2_proofs::plonk::Gate<F> {
    fn name(&self) -> &str {
        self.name()
    }

    fn polynomials(&self) -> &[crate::halo2::Expression<F>] {
        self.polynomials()
    }
}

/// Trait for retrieving information about cell queries.
pub trait QueryInfo {
    /// The kind of query this implementation provides information about.
    type Kind: QueryKind;

    /// Returns the rotation offset.
    fn rotation(&self) -> Rotation;

    /// Returns the index of the column the queried cell belongs to.
    fn column_index(&self) -> usize;
}

/// Trait for retrieving information about a selector.
pub trait SelectorInfo {
    /// Returns the identifier of the selector.
    fn id(&self) -> usize;
}

/// Trait for retrieving information about group annotations.
pub trait GroupInfo {
    /// Returns the inputs of the group.
    fn inputs(&self) -> impl Iterator<Item = Cell> + '_;

    /// Returns the outputs of the group.
    fn outputs(&self) -> impl Iterator<Item = Cell> + '_;
}
