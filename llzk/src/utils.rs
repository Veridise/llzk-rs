use std::{
    ffi::c_void,
    fmt::{self, Formatter},
};

use melior::StringRef;
use mlir_sys::MlirStringRef;

pub trait FromRaw<RawT> {
    /// Constructs Self from RawT via some unsafe function.
    /// # Safety
    /// The raw value must be a valid reference to some MLIR object.
    unsafe fn from_raw(raw: RawT) -> Self;
}

#[allow(dead_code)]
pub(crate) unsafe extern "C" fn print_callback(string: MlirStringRef, data: *mut c_void) {
    unsafe {
        let (formatter, result) = &mut *(data as *mut (&mut Formatter, fmt::Result));

        if result.is_err() {
            return;
        }

        *result = (|| {
            write!(
                formatter,
                "{}",
                StringRef::from_raw(string)
                    .as_str()
                    .map_err(|_| fmt::Error)?
            )
        })();
    }
}

#[macro_export]
macro_rules! ident {
    ($ctx:expr, $name:expr) => {{
        let ctx = $ctx;
        Identifier::new(unsafe { ctx.to_ref() }, $name)
    }};
}
