pub mod array;
pub mod bool;
pub mod constrain;
pub mod felt;
pub mod function;
pub mod global;
pub mod llzk;
pub mod r#struct;

pub mod module {
    use std::ffi::CStr;

    use llzk_sys::LLZK_LANG_ATTR_NAME;
    use melior::ir::{Location, Module, attribute::StringAttribute, operation::OperationMutLike};

    pub fn llzk_module<'c>(location: Location<'c>) -> Module<'c> {
        let mut module = Module::new(location);
        let mut op = module.as_operation_mut();
        let context = unsafe { location.context().to_ref() };
        let attr_name = unsafe { CStr::from_ptr(LLZK_LANG_ATTR_NAME) }
            .to_str()
            .unwrap();
        op.set_attribute(attr_name, StringAttribute::new(context, "llzk").into());
        module
    }
}
