//! `bool` dialect.

mod attrs;
mod ops;

pub use attrs::{CmpPredicate, CmpPredicateAttribute};
pub use ops::{and, assert, eq, ge, gt, le, lt, ne, not, or, xor};
pub use ops::{is_bool_and, is_bool_assert, is_bool_cmp, is_bool_not, is_bool_or, is_bool_xor};

/// Exports the common types of the felt dialect.
pub mod prelude {
    pub use super::attrs::{CmpPredicate, CmpPredicateAttribute};
}
