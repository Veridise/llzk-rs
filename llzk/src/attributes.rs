//! Utilities related to MLIR attributes.

use melior::ir::{Attribute, Identifier};

/// An attribute associated to a name.
pub type NamedAttribute<'c> = (Identifier<'c>, Attribute<'c>);
