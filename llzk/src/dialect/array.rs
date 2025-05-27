use llzk_sys::{
    llzkArrayTypeGet, llzkArrayTypeGetDim, llzkArrayTypeGetElementType, llzkArrayTypeGetNumDims,
    llzkArrayTypeGetWithNumericDims, llzkTypeIsAArrayType, mlirGetDialectHandle__llzk__array__,
};
use melior::{
    dialect::DialectHandle,
    ir::{
        attribute::IntegerAttribute, operation::OperationBuilder, Attribute, Identifier, Location,
        Operation, Type, TypeLike, Value,
    },
};
use mlir_sys::MlirType;

pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__array__()) }
}

pub fn new_with_values<'c>(
    r#type: ArrayType<'c>,
    values: &[Value<'c, '_>],
    location: Location<'c>,
) -> Operation<'c> {
    OperationBuilder::new("array.new", location)
        .add_results(&[r#type.r#type])
        .add_operands(values)
        .add_attributes(&[(
            Identifier::new(unsafe { r#type.context().to_ref() }, "operandSegmentSizes"),
            IntegerAttribute::new(
                Type::index(unsafe { r#type.context().to_ref() }),
                values.len().try_into().unwrap(),
            )
            .into(),
        )])
        .build()
        .unwrap()
}

#[derive(Debug, Eq, PartialEq)]
pub struct ArrayType<'c> {
    r#type: Type<'c>,
}

impl<'c> ArrayType<'c> {
    unsafe fn from_raw(raw: MlirType) -> Self {
        Self {
            r#type: unsafe { Type::from_raw(raw) },
        }
    }

    pub fn new(element_type: Type<'c>, dims: &[Attribute<'c>]) -> Self {
        unsafe {
            Self::from_raw(llzkArrayTypeGet(
                element_type.to_raw(),
                dims.len() as _,
                dims.as_ptr() as *const _,
            ))
        }
    }

    pub fn new_with_dims(element_type: Type<'c>, dims: &[i64]) -> Self {
        unsafe {
            Self::from_raw(llzkArrayTypeGetWithNumericDims(
                element_type.to_raw(),
                dims.len() as _,
                dims.as_ptr() as *const _,
            ))
        }
    }

    pub fn element_type(&self) -> Type<'c> {
        unsafe { Type::from_raw(llzkArrayTypeGetElementType(self.to_raw())) }
    }

    pub fn num_dims(&self) -> isize {
        unsafe { llzkArrayTypeGetNumDims(self.to_raw()) }
    }

    pub fn dim(&self, idx: isize) -> Attribute<'c> {
        unsafe { Attribute::from_raw(llzkArrayTypeGetDim(self.to_raw(), idx)) }
    }
}

impl<'c> TypeLike<'c> for ArrayType<'c> {
    fn to_raw(&self) -> MlirType {
        self.r#type.to_raw()
    }
}

impl<'c> TryFrom<Type<'c>> for ArrayType<'c> {
    type Error = melior::Error;

    fn try_from(t: Type<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkTypeIsAArrayType(t.to_raw()) } {
            Ok(unsafe { Self::from_raw(t.to_raw()) })
        } else {
            Err(Self::Error::TypeExpected("llzk array", t.to_string()))
        }
    }
}

impl<'c> std::fmt::Display for ArrayType<'c> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.r#type, formatter)
    }
}

#[cfg(test)]
mod tests {
    use melior::{
        dialect::arith,
        ir::{operation::OperationLike, Module, Value},
        Context,
    };
    use rstest::rstest;

    use crate::{
        builder::{Builder, OpBuilder},
        dialect::array::new_with_values,
        test::ctx,
    };

    use super::ArrayType;

    #[rstest]
    fn type_new_with_dims(ctx: Context) {
        let builder = Builder::from_ref(&ctx);

        let idx_typ = builder.index_type();
        let arr_typ = ArrayType::new_with_dims(idx_typ.clone(), &[2]);

        assert_eq!(arr_typ.element_type(), idx_typ);
        assert_eq!(arr_typ.num_dims(), 1);
        assert_eq!(arr_typ.dim(0), builder.index_attr(2));
    }

    #[rstest]
    fn op_new_with_values(ctx: Context) {
        //let builder = Builder::from_ref(&ctx);
        //let arr_typ = ArrayType::new_with_dims(builder.index_type(), &[2]);
        //let module = Module::new(builder.unknown_loc());
        //assert_eq!(ctx, module.context());
        //let op_builder = OpBuilder::at_block_begin(module.context(), module.body());
        //let op = op_builder.insert(builder.unknown_loc(), |b, loc| {
        //    let op1 = b.insert(loc, |b, loc| {
        //        arith::constant(b.context_ref(), b.index_attr(1), loc)
        //    });
        //
        //    let op2 = b.insert(loc, |b, loc| {
        //        arith::constant(b.context_ref(), b.index_attr(2), loc)
        //    });
        //
        //    let vals: [Value; 2] = [op1.result(0).unwrap().into(), op2.result(0).unwrap().into()];
        //    new_with_values(arr_typ, &vals, loc)
        //});
        //assert!(op.verify());
    }
}
