//! APIs for the different dialects available in LLZK.

pub mod array;
pub mod bool;
pub mod cast;
pub mod constrain;
pub mod felt;
pub mod function;
pub mod global;
pub mod llzk;
pub mod poly;
pub mod r#struct;
pub mod undef;

/// Functions for working with `builtin.module` in LLZK.
pub mod module {
    use std::ffi::CStr;

    use llzk_sys::LLZK_LANG_ATTR_NAME;
    use melior::ir::{Location, Module, attribute::StringAttribute, operation::OperationMutLike};

    /// Creates a new `builtin.module` operation preconfigured to meet LLZK's specifications.
    pub fn llzk_module<'c>(location: Location<'c>) -> Module<'c> {
        let mut module = Module::new(location);
        let mut op = module.as_operation_mut();
        let ctx = location.context();
        let attr_name = unsafe { CStr::from_ptr(LLZK_LANG_ATTR_NAME) }
            .to_str()
            .unwrap();
        op.set_attribute(
            attr_name,
            StringAttribute::new(unsafe { ctx.to_ref() }, "llzk").into(),
        );
        module
    }
}
