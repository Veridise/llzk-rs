//! Exports the most common types and function in llzk.

pub use crate::context::LlzkContext;
pub use crate::dialect::array::prelude::*;
pub use crate::dialect::felt::prelude::*;
pub use crate::dialect::function::prelude::*;
pub use crate::dialect::llzk::prelude::*;
pub use crate::dialect::module::llzk_module;
pub use crate::dialect::r#struct::prelude::*;
pub use crate::error::Error as LlzkError;
pub use crate::passes as llzk_passes;

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
        add, bit_and, bit_not, bit_or, bit_xor, constant, div, inv, r#mod, mul, neg, shl, shr, sub,
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
/// Exports functions that create operations
pub mod undef {
    pub use crate::dialect::undef::undef;
}

/// melior reexports of commonly used types.
pub use melior::{
    Context, ContextRef, Error as MeliorError, StringRef,
    ir::{
        Location, Region, RegionLike, RegionRef, Value, ValueLike,
        attribute::{
            Attribute, AttributeLike, FlatSymbolRefAttribute, IntegerAttribute, StringAttribute,
        },
        block::{Block, BlockArgument, BlockLike, BlockRef},
        operation::{Operation, OperationLike, OperationMutLike, OperationRef},
        r#type::{FunctionType, IntegerType, Type, TypeLike},
    },
    pass::{OperationPassManager, Pass, PassManager},
};

/// Reexpor of the passes included in melior.
pub mod melior_passes {
    pub use melior::pass::r#async::*;
    pub use melior::pass::conversion::*;
    pub use melior::pass::gpu::*;
    pub use melior::pass::linalg::*;
    pub use melior::pass::sparse_tensor::*;
    pub use melior::pass::transform::*;
}
