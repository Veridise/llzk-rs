//! Structs for handling lookups.

/// Lightweight representation of a lookup that is cheap to copy
#[derive(Debug, Copy, Clone)]
pub struct LookupData<'syn, E> {
    /// Name of the lookup.
    pub name: &'syn str,
    /// Expressions representing the arguments of the lookup.
    pub arguments: &'syn [E],
    /// Expressions representing the columns of the table.
    pub table: &'syn [E],
}
