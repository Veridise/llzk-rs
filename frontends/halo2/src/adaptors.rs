//! Adaptor traits that clients need to implement for communicating with the [`crate::driver::Driver`].

use ff::Field;

use crate::lookups::LookupData;

/// Boxed constraints system adaptor.
pub type CSA<F> = Box<dyn ConstraintSystemAdaptor<F>>;

/// Trait for querying information about the constraint system derived during configuration.
pub trait ConstraintSystemAdaptor<F: Field> {
    /// Returns the list of gates defined in the system.
    fn gates(&self) -> &Vec<crate::halo2::Gate<F>>;

    /// Returns a list with data about the lookups defined in the system.
    fn lookups<'cs>(&'cs self) -> Vec<LookupData<'cs, F>>;
}

/// Temporary implementation of [`ConstraintSystemAdaptor`].
impl<F: Field> ConstraintSystemAdaptor<F> for halo2_proofs::plonk::ConstraintSystem<F> {
    fn gates(&self) -> &Vec<crate::halo2::Gate<F>> {
        self.gates()
    }

    fn lookups<'cs>(&'cs self) -> Vec<LookupData<'cs, F>> {
        self.lookups()
            .iter()
            .map(|a| LookupData {
                name: a.name(),
                inputs: a.input_expressions(),
                table: a.table_expressions(),
            })
            .collect()
    }
}
