#![warn(rustdoc::broken_intra_doc_links)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

use llzk_sys::llzkRegisterAllDialects;
use melior::dialect::DialectRegistry;

pub mod builder;
pub mod context;
mod diagnostics;
pub mod dialect;
pub mod error;
mod macros;
pub mod operation;
pub mod passes;
pub mod prelude;
pub mod symbol_ref;
#[cfg(test)]
mod test;
pub mod utils;
pub mod value_range;

/// Adds all LLZK dialects into the given registry.
pub fn register_all_llzk_dialects(registry: &DialectRegistry) {
    unsafe { llzkRegisterAllDialects(registry.to_raw()) }
}
