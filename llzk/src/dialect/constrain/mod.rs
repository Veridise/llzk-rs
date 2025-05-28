use llzk_sys::mlirGetDialectHandle__llzk__constrain__;
use melior::{
    dialect::DialectHandle,
    ir::{operation::OperationBuilder, Location, Operation, Value},
};

pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__constrain__()) }
}

pub fn eq<'c>(location: Location<'c>, lhs: Value<'c, '_>, rhs: Value<'c, '_>) -> Operation<'c> {
    OperationBuilder::new("constrain.eq", location)
        .add_operands(&[lhs, rhs])
        .build()
        .expect("valid operation")
}

pub fn r#in<'c>(location: Location<'c>, lhs: Value<'c, '_>, rhs: Value<'c, '_>) -> Operation<'c> {
    OperationBuilder::new("constrain.in", location)
        .add_operands(&[lhs, rhs])
        .build()
        .expect("valid operation")
}
