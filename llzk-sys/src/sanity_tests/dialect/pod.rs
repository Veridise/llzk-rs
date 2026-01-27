use crate::{
    llzkPodTypeGet, llzkPodTypeGetNumRecords, llzkPodTypeGetRecords, llzkPodTypeLookupRecord,
    llzkRecordAttrGet, llzkRecordAttrGetName, llzkRecordAttrGetType,
    mlirGetDialectHandle__llzk__pod__,
    sanity_tests::{
        TestContext, context, str_ref,
        typing::{IndexType, index_type},
    },
};
use mlir_sys::{MlirAttribute, MlirType};
use rstest::rstest;
use std::ptr::null;

#[test]
fn test_mlir_get_dialect_handle_llzk_pod() {
    unsafe {
        mlirGetDialectHandle__llzk__pod__();
    }
}

#[rstest]
fn test_llzk_record_attr_get(index_type: IndexType) {
    unsafe {
        let s = str_ref("rec_name");
        let a = llzkRecordAttrGet(s, index_type.t);
        assert_ne!(a.ptr, null());
    }
}

#[rstest]
fn test_llzk_record_attr_name(index_type: IndexType) {
    unsafe {
        let s = str_ref("rec_name");
        let a = llzkRecordAttrGet(s, index_type.t);
        let n = llzkRecordAttrGetName(a);
        assert_ne!(n.data, null());
        assert_ne!(n.length, 0);
    }
}

#[rstest]
fn test_llzk_record_attr_type(index_type: IndexType) {
    unsafe {
        let s = str_ref("rec_name");
        let a = llzkRecordAttrGet(s, index_type.t);
        let t = llzkRecordAttrGetType(a);
        assert_ne!(t.ptr, null());
        assert_eq!(t.ptr, index_type.t.ptr);
    }
}

#[rstest]
fn test_llzk_pod_type_get_empty(context: TestContext) {
    unsafe {
        let t = llzkPodTypeGet(context.ctx, 0, null());
        assert_ne!(t.ptr, null());
    }
}

#[rstest]
fn test_llzk_pod_type_get_non_empty(context: TestContext, index_type: IndexType) {
    unsafe {
        let records = vec![
            llzkRecordAttrGet(str_ref("rec1"), index_type.t),
            llzkRecordAttrGet(str_ref("rec2"), index_type.t),
        ];
        let t = llzkPodTypeGet(context.ctx, records.len() as isize, records.as_ptr());
        assert_ne!(t.ptr, null());
    }
}
#[rstest]
fn test_llzk_pod_type_num_records(context: TestContext) {
    unsafe {
        let t = llzkPodTypeGet(context.ctx, 0, null());
        assert_ne!(t.ptr, null());
        let n = llzkPodTypeGetNumRecords(t);
        assert_eq!(n, 0);
    }
}

#[rstest]
fn test_llzk_pod_type_lookup_record(context: TestContext, index_type: IndexType) {
    unsafe {
        let records = vec![
            llzkRecordAttrGet(str_ref("rec1"), index_type.t),
            llzkRecordAttrGet(str_ref("rec2"), index_type.t),
        ];
        let t: MlirType = llzkPodTypeGet(context.ctx, records.len() as isize, records.as_ptr());
        assert_ne!(t.ptr, null());
        let num = llzkPodTypeGetNumRecords(t);
        assert_eq!(num, 2);
        let r_ty = llzkPodTypeLookupRecord(t, str_ref("rec1"));
        assert_ne!(r_ty.ptr, null());
        let r_ty = llzkPodTypeLookupRecord(t, str_ref("invalid"));
        assert_eq!(r_ty.ptr, null());
    }
}

#[rstest]
fn test_llzk_pod_type_get_records(context: TestContext, index_type: IndexType) {
    unsafe {
        let records = vec![
            llzkRecordAttrGet(str_ref("rec1"), index_type.t),
            llzkRecordAttrGet(str_ref("rec2"), index_type.t),
        ];
        let t: MlirType = llzkPodTypeGet(context.ctx, records.len() as isize, records.as_ptr());
        assert_ne!(t.ptr, null());

        let num = llzkPodTypeGetNumRecords(t);
        let mut raw = vec![MlirAttribute { ptr: null() }; num.try_into().unwrap()];
        llzkPodTypeGetRecords(t, raw.as_mut_ptr());
        assert_eq!(raw.len(), 2);
    }
}
