//! Exports the most common types and function in llzk.

pub use crate::context::LlzkContext;
pub use crate::dialect::array::prelude::*;
pub use crate::dialect::bool::prelude::*;
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
    pub use crate::dialect::array::{extract, insert, len, new, read, write};
    pub use crate::dialect::array::{
        is_array_extract, is_array_insert, is_array_len, is_array_new, is_array_read,
        is_array_type, is_array_write,
    };
}
/// Exports functions from the 'bool' dialect
pub mod bool {
    pub use crate::dialect::bool::{and, assert, eq, ge, gt, le, lt, ne, not, or, xor};
    pub use crate::dialect::bool::{
        is_bool_and, is_bool_assert, is_bool_cmp, is_bool_not, is_bool_or, is_bool_xor,
    };
}

/// Exports functions from the 'cast' dialect
pub mod cast {
    pub use crate::dialect::cast::{is_cast_tofelt, is_cast_toindex, is_cast_toint};
    pub use crate::dialect::cast::{tofelt, toindex, toint};
}

/// Exports functions from the 'constrain' dialect
pub mod constrain {
    pub use crate::dialect::constrain::{eq, r#in};
    pub use crate::dialect::constrain::{is_constrain_eq, is_constrain_in};
}
/// Exports functions from the 'felt' dialect
pub mod felt {
    pub use crate::dialect::felt::{
        add, bit_and, bit_not, bit_or, bit_xor, constant, div, inv, r#mod, mul, neg, pow, shl, shr,
        sub,
    };
    pub use crate::dialect::felt::{
        is_felt_add, is_felt_bit_and, is_felt_bit_not, is_felt_bit_or, is_felt_bit_xor,
        is_felt_const, is_felt_div, is_felt_inv, is_felt_mod, is_felt_mul, is_felt_neg,
        is_felt_pow, is_felt_shl, is_felt_shr, is_felt_sub, is_felt_type,
    };
}
/// Exports functions from the 'function' dialect
pub mod function {
    pub use crate::dialect::function::{call, def, r#return};
    pub use crate::dialect::function::{is_func_call, is_func_def, is_func_return};
}
/// Exports functions from the 'global' dialect
pub mod global {
    pub use crate::dialect::global::{def, read, write};
    pub use crate::dialect::global::{is_global_def, is_global_read, is_global_write};
}
/// Exports functions from the 'poly' dialect
pub mod poly {
    pub use crate::dialect::poly::ops::{is_read_const_op, read_const};
    pub use crate::dialect::poly::r#type::{TVarType, is_type_variable};
}
/// Exports functions from the 'struct' dialect
pub mod r#struct {
    pub use crate::dialect::r#struct::helpers;
    pub use crate::dialect::r#struct::{def, field, new, readf, readf_with_offset, writef};
    pub use crate::dialect::r#struct::{
        is_struct_def, is_struct_field, is_struct_new, is_struct_readf, is_struct_type,
        is_struct_writef,
    };
}
/// Exports functions from the 'undef' dialect
pub mod undef {
    pub use crate::dialect::undef::{is_undef_op, undef};
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
            Operation, OperationLike, OperationMutLike, OperationRef, OperationRefMut,
            OperationResult, WalkOrder, WalkResult,
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
    /// Exports functions from the 'scf' dialect and extensions for LLZK.
    pub mod scf {
        pub use crate::dialect::scf_ext::{
            is_scf_condition, is_scf_for, is_scf_if, is_scf_while, is_scf_yield,
        };
        pub use melior::dialect::scf::*;
    }
    pub use melior::dialect::index;
}
