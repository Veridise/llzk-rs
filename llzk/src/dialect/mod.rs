pub mod array;
pub mod bool;
pub mod constrain;
pub mod felt;
pub mod function;
pub mod global;
pub mod r#struct;

pub mod module {
    use std::ffi::CStr;

    use llzk_sys::LLZK_LANG_ATTR_NAME;
    use melior::ir::{attribute::StringAttribute, operation::OperationMutLike, Location, Module};

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
