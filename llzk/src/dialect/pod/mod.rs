//! `pod` dialect.

pub mod attrs;
pub mod ops;
pub mod r#type;
pub use ops::is_pod_new;
pub use ops::new;

use llzk_sys::mlirGetDialectHandle__llzk__pod__;
use melior::dialect::DialectHandle;

/// Returns a handle to the `pod` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__pod__()) }
}

/// Exports the common types and records of the pod dialect.
pub mod prelude {
    pub use super::attrs::PodRecordAttribute;
    pub use super::r#type::{PodType, is_pod_type};
}
