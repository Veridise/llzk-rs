mod attrs;
mod ops;

pub use attrs::{CmpPredicate, CmpPredicateAttribute};
pub use ops::{and, assert, eq, ge, gt, le, lt, ne, not, or, xor};

/// Exports the common types of the felt dialect.
pub mod prelude {
    pub use super::attrs::{CmpPredicate, CmpPredicateAttribute};
}
