//! Exports the most common types and function in llzk.

pub use crate::context::LlzkContext;
pub use crate::dialect::array::prelude::*;
pub use crate::dialect::felt::prelude::*;
pub use crate::dialect::function::prelude::*;
pub use crate::dialect::llzk::prelude::*;
pub use crate::dialect::module::llzk_module;
pub use crate::dialect::r#struct::prelude::*;
pub use crate::error::Error as LlzkError;
pub use crate::operation::{replace_uses_of_with, verify_operation, verify_operation_with_diags};
pub use crate::passes as llzk_passes;
pub use crate::symbol_ref::{SymbolRefAttrLike, SymbolRefAttribute};
pub use crate::utils::IntoRef;

/// Exports functions from the 'array' dialect
pub mod array {
    pub use crate::dialect::array::{extract, insert, new, read, write};
}
/// Exports functions from the 'bool' dialect
pub mod bool {
    pub use crate::dialect::bool::{and, assert, eq, ge, gt, le, lt, ne, not, or, xor};
}

/// Exports functions from the 'cast' dialect
pub mod cast {
    pub use crate::dialect::cast::{tofelt, toindex, toint};
}

/// Exports functions from the 'constrain' dialect
pub mod constrain {
    pub use crate::dialect::constrain::{eq, r#in};
}
/// Exports functions from the 'felt' dialect
pub mod felt {
    pub use crate::dialect::felt::{
        add, bit_and, bit_not, bit_or, bit_xor, constant, div, inv, r#mod, mul, neg, pow, shl, shr,
        sub,
    };
}
/// Exports functions from the 'function' dialect
pub mod function {
    pub use crate::dialect::function::{call, def, r#return};
}
/// Exports functions from the 'global' dialect
pub mod global {
    pub use crate::dialect::global::{def, read, write};
}
/// Exports functions from the 'poly' dialect
pub mod poly {
    pub use crate::dialect::poly::r#type::TVarType;
    pub use crate::dialect::poly::{is_read_const_op, read_const};
}
/// Exports functions from the 'struct' dialect
pub mod r#struct {
    pub use crate::dialect::r#struct::helpers;
    pub use crate::dialect::r#struct::{def, field, new, readf, readf_with_offset, writef};
}
/// Exports functions from the 'undef' dialect
pub mod undef {
    pub use crate::dialect::undef::undef;
}

/// melior reexports of commonly used types.
pub use melior::{
    Context, ContextRef, Error as MeliorError, StringRef,
    ir::{
        Location, Module, Region, RegionLike, RegionRef, Value, ValueLike,
        attribute::{
            Attribute, AttributeLike, BoolAttribute, FlatSymbolRefAttribute, IntegerAttribute,
            StringAttribute, TypeAttribute,
        },
        block::{Block, BlockArgument, BlockLike, BlockRef},
        operation::{
            Operation, OperationLike, OperationMutLike, OperationRef, OperationRefMut, WalkOrder,
            WalkResult,
        },
        r#type::{FunctionType, IntegerType, Type, TypeLike},
    },
    pass::{OperationPassManager, Pass, PassManager},
};

/// Reexport of the passes included in melior.
pub mod melior_passes {
    pub use melior::pass::r#async::*;
    pub use melior::pass::conversion::*;
    pub use melior::pass::gpu::*;
    pub use melior::pass::linalg::*;
    pub use melior::pass::sparse_tensor::*;
    pub use melior::pass::transform::*;
}

/// Reexport of the dialects included in melior.
pub mod melior_dialects {
    pub use melior::dialect::arith;
    pub use melior::dialect::index;
    pub use melior::dialect::scf;
}
