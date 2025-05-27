use std::ffi::{c_char, CStr};

use crate::{
    LLZK_COMPONENT_NAME_MAIN, LLZK_COMPONENT_NAME_SIGNAL, LLZK_FUNC_NAME_COMPUTE,
    LLZK_FUNC_NAME_CONSTRAIN, LLZK_LANG_ATTR_NAME,
};

fn unwrap(s: *const c_char) -> String {
    unsafe { CStr::from_ptr(s.clone()) }
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn test_llzk_constants() {
    assert_eq!(unwrap(unsafe { LLZK_COMPONENT_NAME_SIGNAL }), "Signal");
    assert_eq!(unwrap(unsafe { LLZK_COMPONENT_NAME_MAIN }), "Main");
    assert_eq!(unwrap(unsafe { LLZK_FUNC_NAME_COMPUTE }), "compute");
    assert_eq!(unwrap(unsafe { LLZK_FUNC_NAME_CONSTRAIN }), "constrain");
    assert_eq!(unwrap(unsafe { LLZK_LANG_ATTR_NAME }), "veridise.lang");
}
