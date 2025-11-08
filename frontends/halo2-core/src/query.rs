//! Types and traits related to cell queries.

mod sealed {
    /// Sealed trait pattern to avoid clients implementing the trait [`super::QueryKind`] on
    /// external types.
    pub trait QK {}
}

/// Marker trait for defining the kind of a query.
pub trait QueryKind: sealed::QK {}

/// Marker for fixed cell queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fixed;

impl sealed::QK for Fixed {}
impl QueryKind for Fixed {}

/// Marker for advice cell queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Advice;

impl sealed::QK for Advice {}
impl QueryKind for Advice {}

/// Marker for instance cell queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Instance;

impl sealed::QK for Instance {}
impl QueryKind for Instance {}
