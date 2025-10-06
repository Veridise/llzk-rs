mod attrs;

pub use attrs::PublicAttribute;

/// Exports the common types of the llzk dialect.
pub mod prelude {
    pub use super::attrs::PublicAttribute;
}
