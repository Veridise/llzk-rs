//! `pod` dialect operations and helper functions.

use super::r#type::PodType;
use crate::{
    builder::{OpBuilder, OpBuilderLike},
    ident,
    prelude::FlatSymbolRefAttribute,
};
use llzk_sys::{LlzkRecordValue, llzkNewPodOpBuild, llzkNewPodOpBuildInferredFromInitialValues};
use melior::StringRef;
use melior::ir::{
    Location, Operation, Type, TypeLike, Value, ValueLike,
    operation::{OperationBuilder, OperationLike},
};
use std::marker::PhantomData;

/// Wrapper around a `LlzkRecordValue`, used to initialize fields in a `pod.new` operation.
#[derive(Debug)]
pub struct RecordValue<'c, 'a> {
    raw: LlzkRecordValue,
    _context: PhantomData<Value<'c, 'a>>,
}

impl<'c, 'a> RecordValue<'c, 'a> {
    /// Creates a new record value.
    pub fn new(name: StringRef, value: Value<'c, 'a>) -> Self {
        Self {
            raw: LlzkRecordValue {
                name: name.to_raw(),
                value: value.to_raw(),
            },
            _context: PhantomData,
        }
    }

    /// Returns the raw representation of the record value.
    pub fn to_raw(&self) -> LlzkRecordValue {
        self.raw
    }

    /// Creates the record value from a raw pointer.
    pub fn from_raw(raw: LlzkRecordValue) -> Self {
        Self {
            raw,
            _context: PhantomData,
        }
    }
}

/// Creates a 'pod.new' operation from a list of initialization values. If the optional type
/// of the result pod is not given, it will be inferred from the provided initialization values.
pub fn new<'c, 'a>(
    builder: &OpBuilder<'c>,
    location: Location<'c>,
    values: &[RecordValue<'c, 'a>],
    r#type: Option<PodType<'c>>,
) -> Operation<'c> {
    let raw_values: Vec<_> = values.iter().map(RecordValue::to_raw).collect();
    if let Some(r#type) = r#type {
        unsafe {
            Operation::from_raw(llzkNewPodOpBuild(
                builder.to_raw(),
                location.to_raw(),
                r#type.to_raw(),
                raw_values.len() as isize,
                raw_values.as_ptr(),
            ))
        }
    } else {
        unsafe {
            Operation::from_raw(llzkNewPodOpBuildInferredFromInitialValues(
                builder.to_raw(),
                location.to_raw(),
                raw_values.len() as isize,
                raw_values.as_ptr(),
            ))
        }
    }
}

/// Return `true` iff the given op is `pod.new`.
#[inline]
pub fn is_pod_new<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "pod.new")
}

/// Creates a 'pod.read' operation.
pub fn read<'c>(
    location: Location<'c>,
    pod_ref: Value<'c, '_>,
    record_name: FlatSymbolRefAttribute<'c>,
    result: Type<'c>,
) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("pod.read", location)
        .add_attributes(&[(ident!(ctx, "record_name"), record_name.into())])
        .add_operands(&[pod_ref])
        .add_results(&[result])
        .build()
        .expect("valid operation")
}

/// Return `true` iff the given op is `pod.read`.
#[inline]
pub fn is_pod_read<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "pod.read")
}

/// Creates a 'pod.write' operation.
pub fn write<'c>(
    location: Location<'c>,
    pod_ref: Value<'c, '_>,
    record_name: FlatSymbolRefAttribute<'c>,
    rvalue: Value<'c, '_>,
) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("pod.write", location)
        .add_attributes(&[(ident!(ctx, "record_name"), record_name.into())])
        .add_operands(&[pod_ref, rvalue])
        .build()
        .expect("valid operation")
}

/// Return `true` iff the given op is `pod.write`.
#[inline]
pub fn is_pod_write<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "pod.write")
}
