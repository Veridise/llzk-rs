use llzk_sys::llzkRegisterAllDialects;
use melior::dialect::DialectRegistry;

pub mod builder;
pub mod context;
mod diagnostics;
pub mod dialect;
pub mod error;
mod macros;
pub mod symbol_ref;
#[cfg(test)]
mod test;
pub mod utils;
pub mod value_range;

/// Adds all LLZK dialects into the given registry.
pub fn register_all_llzk_dialects(registry: &DialectRegistry) {
    unsafe { llzkRegisterAllDialects(registry.to_raw()) }
}

/// Exports the most common types and function in llzk.
pub mod prelude {
    pub use crate::context::LlzkContext;
    pub use crate::dialect::array::prelude::*;
    pub use crate::dialect::felt::prelude::*;
    pub use crate::dialect::function::prelude::*;
    pub use crate::dialect::llzk::prelude::*;
    pub use crate::dialect::module::llzk_module;
    pub use crate::dialect::r#struct::prelude::*;
    pub use crate::error::Error as LlzkError;

    /// Exports functions that create operations
    pub mod array {
        pub use crate::dialect::array::{extract, insert, new, read, write};
    }
    /// Exports functions that create operations
    pub mod bool {
        pub use crate::dialect::bool::{and, assert, eq, ge, gt, le, lt, ne, not, or, xor};
    }
    /// Exports functions that create operations
    pub mod constrain {
        pub use crate::dialect::constrain::{eq, r#in};
    }
    /// Exports functions that create operations
    pub mod felt {
        pub use crate::dialect::felt::{
            add, bit_and, bit_not, bit_or, bit_xor, constant, div, inv, mul, neg, r#mod, shl, shr,
            sub,
        };
    }
    /// Exports functions that create operations
    pub mod function {
        pub use crate::dialect::function::{call, def, r#return};
    }
    /// Exports functions that create operations
    pub mod global {
        pub use crate::dialect::global::{def, read, write};
    }
    /// Exports functions that create operations
    pub mod r#struct {
        pub use crate::dialect::r#struct::helpers;
        pub use crate::dialect::r#struct::{def, field, new, readf, readf_with_offset, writef};
    }

    // melior reexports of commonly used types.
    pub use melior::ir::block::{Block, BlockLike};
    pub use melior::ir::operation::OperationLike;
    pub use melior::ir::operation::OperationMutLike;
    pub use melior::ir::RegionLike;
}
