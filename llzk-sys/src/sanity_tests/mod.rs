use mlir_sys::*;
use rstest::{fixture, rstest};
use std::ffi::CString;

use crate::llzkRegisterAllDialects;

mod builder;
mod constants;
mod dialect;
mod init_dialects;
mod transforms;
mod typing;
mod validators;

macro_rules! define_fixture {
    ($name:ident, $fixture_type:ident, $field:ident, $typ:ty, $create:ident, $destroy:ident) => {
        pub struct $fixture_type {
            $field: $typ,
        }

        impl Drop for $fixture_type {
            fn drop(&mut self) {
                unsafe { $destroy(self.$field) }
            }
        }

        #[fixture]
        pub fn $name() -> $fixture_type {
            $fixture_type {
                $field: unsafe { $create() },
            }
        }
    };

    ($name:ident, $fixture_type:ident, $field:ident, $typ:ty, $create:ident, $destroy:ident, $and_then:expr) => {
        pub struct $fixture_type {
            $field: $typ,
        }

        impl Drop for $fixture_type {
            fn drop(&mut self) {
                unsafe { $destroy(self.$field) }
            }
        }

        #[fixture]
        pub fn $name() -> $fixture_type {
            let fixture = $fixture_type {
                $field: unsafe { $create() },
            };
            ($and_then)(&fixture);
            fixture
        }
    };
}

define_fixture!(
    context,
    TestContext,
    ctx,
    MlirContext,
    mlirContextCreate,
    mlirContextDestroy,
    |context| { load_llzk_dialects(context) }
);

impl AsRef<MlirContext> for TestContext {
    fn as_ref(&self) -> &MlirContext {
        &self.ctx
    }
}

define_fixture!(
    registry,
    TestRegistry,
    registry,
    MlirDialectRegistry,
    mlirDialectRegistryCreate,
    mlirDialectRegistryDestroy
);

pub fn load_llzk_dialects<Ctx: AsRef<MlirContext>>(ctx: &Ctx) {
    unsafe {
        let registry = mlirDialectRegistryCreate();
        let ctx = *ctx.as_ref();
        mlirRegisterAllDialects(registry);
        llzkRegisterAllDialects(registry);
        mlirContextAppendDialectRegistry(ctx, registry);

        mlirContextLoadAllAvailableDialects(ctx);
        mlirDialectRegistryDestroy(registry);
    }
}

pub fn str_ref(s: &'static str) -> MlirStringRef {
    MlirStringRef {
        data: (s.as_ptr() as *const ::core::ffi::c_char),
        length: s.len(),
    }
}

#[rstest]
fn test_context(context: TestContext) {
    unsafe {
        assert!(mlirContextEqual(context.ctx, context.ctx));
    }
}

#[test]
fn create_string() {
    unsafe {
        let string = CString::new("Hello, world!").unwrap();

        mlirStringRefCreateFromCString(string.as_ptr());
    }
}

#[rstest]
fn test_location(context: TestContext, registry: TestRegistry) {
    unsafe {
        mlirContextAppendDialectRegistry(context.ctx, registry.registry);
        mlirRegisterAllDialects(registry.registry);

        let location = mlirLocationUnknownGet(context.ctx);
        let string = CString::new("newmod").unwrap();
        let reference = mlirStringRefCreateFromCString(string.as_ptr());

        mlirOperationStateGet(reference, location);
    }
}
