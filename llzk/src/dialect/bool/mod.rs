mod attrs;
mod ops;

pub use attrs::{CmpPredicate, CmpPredicateAttribute};
pub use ops::{eq, ge, gt, le, lt, ne};

/// Exports the common types of the felt dialect.
pub mod prelude {
    pub use super::attrs::{CmpPredicate, CmpPredicateAttribute};
}
