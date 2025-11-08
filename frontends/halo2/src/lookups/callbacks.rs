//! Structs for handling lookups from the client side.

use std::borrow::Cow;

use crate::{
    ir::stmt::IRStmt,
    lookups::table::LookupTableGenerator,
    temps::{ExprOrTemp, Temps},
};
use anyhow::Result;
use ff::Field;

use super::Lookup;

/// Callback trait for defering to the client how to handle the logic of a lookup.
pub trait LookupCallbacks<F: Field, E> {
    /// Called on the list of lookups the circuit defines.
    ///
    /// While generating IR in a circuit with multiple lookups it could be the case that two
    /// lookups are related. For example, the circuit could call the same lookup in the same row
    /// for two values. The client that is extracting the circuit may want to handle these pairs of
    /// lookups in a special manner. This method enables the possibility for callbacks of handling
    /// the lookups in the circuit as a whole. With only calls to [`LookupCallbacks::on_lookup`]
    /// for each lookup is not possible to do that since the callback would receive each
    /// lookup indepedently.
    ///
    /// For example, consider a lookup for a sha256 implementation that returns the plain and
    /// spreaded version of a value (i.e. for 5 the spreaded value would be 17) and for each row where
    /// the lookup is enabled it invokes it twice (returning spreaded values `x` and `y`).
    /// For verifying with Picus, it helps annotating that if `x + 2*y` is deterministic, then `x`
    /// and `y` are deterministic. Emitting IR that encodes that axiom requires working with both
    /// lookups (each would be a different [`Lookup`]) at the same time.
    ///
    /// The implementation of this method is optional if the callback does not need to do any
    /// inter-lookup work and by default loops over the lookups and calls [`LookupCallbacks::on_lookup`] on each.
    fn on_lookups<'syn>(
        &self,
        lookups: &'syn [Lookup<E>],
        tables: &[&dyn LookupTableGenerator<F>],
        temps: &mut Temps,
    ) -> Result<IRStmt<ExprOrTemp<Cow<'syn, E>>>>
    where
        E: Clone,
    {
        lookups
            .iter()
            .zip(tables.iter())
            .map(|(lookup, table)| self.on_lookup(lookup, *table, temps))
            .collect()
    }

    /// Called on each lookup the circuit defines.
    fn on_lookup<'syn>(
        &self,
        lookup: &'syn Lookup<E>,
        table: &dyn LookupTableGenerator<F>,
        temps: &mut Temps,
    ) -> Result<IRStmt<ExprOrTemp<Cow<'syn, E>>>>
    where
        E: Clone;
}

pub(crate) struct DefaultLookupCallbacks;

impl<F: Field, E: Clone> LookupCallbacks<F, E> for DefaultLookupCallbacks {
    #[allow(unused_variables)]
    fn on_lookup<'syn>(
        &self,
        lookup: &'syn Lookup<E>,
        table: &dyn LookupTableGenerator<F>,
        temps: &mut Temps,
    ) -> Result<IRStmt<ExprOrTemp<Cow<'syn, E>>>> {
        panic!("Target circuit has lookups but their behaviour was not specified");
    }
}
