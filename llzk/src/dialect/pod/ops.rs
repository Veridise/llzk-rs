//! `pod` dialect operations and helper functions.

use super::r#type::PodType;
use crate::{builder::OpBuilder, ident, prelude::FlatSymbolRefAttribute};
use melior::ir::{
    Location, Operation, Type, Value,
    operation::{OperationBuilder, OperationLike},
};

/// Creates a 'pod.new' operation.
pub fn new<'c>(
    builder: &OpBuilder<'c>,
    location: Location<'c>,
    r#type: PodType<'c>,
) -> Operation<'c> {
    todo!()
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
