use std::{fmt, ops::Deref, ptr::null_mut};

use llzk_sys::{
    llzkFieldDefOpGetHasPublicAttr, llzkFieldDefOpSetPublicAttr, llzkFieldReadOpBuild,
    llzkOperationIsAFieldDefOp, llzkOperationIsAStructDefOp, llzkStructDefOpGetComputeFuncOp,
    llzkStructDefOpGetConstrainFuncOp, llzkStructDefOpGetFieldDef, llzkStructDefOpGetFieldDefs,
    llzkStructDefOpGetHasColumns, llzkStructDefOpGetHasParamName,
    llzkStructDefOpGetIsMainComponent, llzkStructDefOpGetNumFieldDefs, llzkStructDefOpGetType,
    llzkStructDefOpGetTypeWithParams,
};
use melior::{
    ir::{
        attribute::{ArrayAttribute, FlatSymbolRefAttribute, TypeAttribute},
        operation::{OperationBuilder, OperationLike},
        Attribute, AttributeLike, Identifier, Location, Operation, OperationRef, Region, Type,
        TypeLike, Value, ValueLike,
    },
    LogicalResult, StringRef,
};
use mlir_sys::MlirOperation;

use crate::{
    builder::{OpBuilder, OpBuilderLike},
    dialect::function::FuncDefOpRef,
    error::Error,
    ident,
};

use super::StructType;

//===----------------------------------------------------------------------===//
// StructDefOpLike
//===----------------------------------------------------------------------===//

/// Defines the public API of the 'struct.def' op.
pub trait StructDefOpLike<'c: 'a, 'a>: OperationLike<'c, 'a> {
    /// Returns the associated StructType to this op using the const params defined by the op.
    fn r#type(&self) -> StructType<'c> {
        unsafe { Type::from_raw(llzkStructDefOpGetType(self.to_raw())) }
            .try_into()
            .expect("StructDefOpLike::type error")
    }

    /// Returns the associated StructType to this op using the given const params instead of the
    /// parameters defined by the op.
    fn type_with_params(&self, params: ArrayAttribute<'c>) -> StructType<'c> {
        unsafe {
            Type::from_raw(llzkStructDefOpGetTypeWithParams(
                self.to_raw(),
                params.to_raw(),
            ))
        }
        .try_into()
        .expect("StructDefOpLike::type error")
    }

    /// Returns the operation that defines the field with the given name, if present.
    fn get_field_def(&self, name: &str) -> Option<FieldDefOpRef<'c, '_>> {
        let name = StringRef::new(name);
        let raw_op = unsafe { llzkStructDefOpGetFieldDef(self.to_raw(), name.to_raw()) };
        if raw_op.ptr == null_mut() {
            return None;
        }
        Some(
            unsafe { OperationRef::from_raw(raw_op) }
                .try_into()
                .expect("op of type 'struct.field'"),
        )
    }

    /// Fills the given array with the FieldDefOp operations inside this struct.  
    fn get_field_defs(&self) -> Vec<FieldDefOpRef<'c, '_>> {
        let num_fields = unsafe { llzkStructDefOpGetNumFieldDefs(self.to_raw()) };
        let mut raw_ops: Vec<MlirOperation> = Default::default();
        raw_ops.reserve(num_fields.try_into().unwrap());
        unsafe { llzkStructDefOpGetFieldDefs(self.to_raw(), raw_ops.as_mut_ptr()) };
        raw_ops
            .into_iter()
            .map(|op| {
                unsafe { OperationRef::from_raw(op) }
                    .try_into()
                    .expect("op of type 'struct.field")
            })
            .collect()
    }

    /// Returns true if the struct has fields marked as columns.
    fn has_columns(&self) -> LogicalResult {
        LogicalResult::from_raw(unsafe { llzkStructDefOpGetHasColumns(self.to_raw()) })
    }

    /// Returns the FuncDefOp operation that defines the witness computation of the struct.
    fn get_compute_func<'b>(&self) -> Option<FuncDefOpRef<'c, 'b>> {
        let raw_op = unsafe { llzkStructDefOpGetComputeFuncOp(self.to_raw()) };
        if raw_op.ptr == null_mut() {
            return None;
        }
        Some(
            unsafe { OperationRef::from_raw(raw_op) }
                .try_into()
                .expect("op of type 'function.def'"),
        )
    }

    /// Returns the FuncDefOp operation that defines the constraints of the struct.
    fn get_constrain_func<'b>(&self) -> Option<FuncDefOpRef<'c, 'b>> {
        let raw_op = unsafe { llzkStructDefOpGetConstrainFuncOp(self.to_raw()) };
        if raw_op.ptr == null_mut() {
            return None;
        }
        Some(
            unsafe { OperationRef::from_raw(raw_op) }
                .try_into()
                .expect("op of type 'function.def'"),
        )
    }

    /// Returns true if the struct has a parameter that with the given name.
    fn has_param_name(&self, name: &str) -> bool {
        let name = StringRef::new(name);
        unsafe { llzkStructDefOpGetHasParamName(self.to_raw(), name.to_raw()) }
    }

    /// Returns a StringAttr with the fully qualified name of the struct.
    fn get_fully_qualified_name(&self) -> Attribute<'_> {
        todo!("melior does not have a SymbolRefAttribute type")
    }

    /// Returns true if the struct is the main entry point of the circuit.
    fn is_main_component(&self) -> bool {
        unsafe { llzkStructDefOpGetIsMainComponent(self.to_raw()) }
    }
}

//===----------------------------------------------------------------------===//
// StructDefOp
//===----------------------------------------------------------------------===//

/// Represents an owned 'struct.def' op.
pub struct StructDefOp<'c> {
    inner: Operation<'c>,
}

impl StructDefOp<'_> {
    pub unsafe fn from_raw(raw: MlirOperation) -> Self {
        unsafe {
            Self {
                inner: Operation::from_raw(raw),
            }
        }
    }
}

impl<'a, 'c: 'a> OperationLike<'c, 'a> for StructDefOp<'c> {
    fn to_raw(&self) -> MlirOperation {
        self.inner.to_raw()
    }
}

impl<'a, 'c: 'a> StructDefOpLike<'c, 'a> for StructDefOp<'c> {}

impl<'c> fmt::Display for StructDefOp<'c> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'c> Deref for StructDefOp<'c> {
    type Target = Operation<'c>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'c> Into<Operation<'c>> for StructDefOp<'c> {
    fn into(self) -> Operation<'c> {
        self.inner
    }
}

impl<'c> TryFrom<Operation<'c>> for StructDefOp<'c> {
    type Error = Error;

    fn try_from(op: Operation<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkOperationIsAStructDefOp(op.to_raw()) } {
            Ok(unsafe { Self::from_raw(op.to_raw()) })
        } else {
            Err(Self::Error::OperationExpected("struct.def", op.to_string()))
        }
    }
}

//===----------------------------------------------------------------------===//
// StructDefOpRef
//===----------------------------------------------------------------------===//

/// Represents a non-owned reference to a 'struct.def' op.
#[derive(Clone, Copy)]
pub struct StructDefOpRef<'c, 'a> {
    inner: OperationRef<'c, 'a>,
}

impl StructDefOpRef<'_, '_> {
    pub unsafe fn from_raw(raw: MlirOperation) -> Self {
        unsafe {
            Self {
                inner: OperationRef::from_raw(raw),
            }
        }
    }
}

impl<'a, 'c: 'a> OperationLike<'c, 'a> for StructDefOpRef<'c, 'a> {
    fn to_raw(&self) -> MlirOperation {
        self.inner.to_raw()
    }
}

impl<'a, 'c: 'a> StructDefOpLike<'c, 'a> for StructDefOpRef<'c, 'a> {}

impl fmt::Display for StructDefOpRef<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'a, 'c: 'a> Deref for StructDefOpRef<'c, 'a> {
    type Target = OperationRef<'c, 'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, 'c: 'a> Into<OperationRef<'c, 'a>> for StructDefOpRef<'c, 'a> {
    fn into(self) -> OperationRef<'c, 'a> {
        self.inner
    }
}

impl<'a, 'c: 'a> TryFrom<OperationRef<'c, 'a>> for StructDefOpRef<'c, 'a> {
    type Error = Error;

    fn try_from(op: OperationRef<'c, 'a>) -> Result<Self, Self::Error> {
        if unsafe { llzkOperationIsAStructDefOp(op.to_raw()) } {
            Ok(unsafe { Self::from_raw(op.to_raw()) })
        } else {
            Err(Self::Error::OperationExpected("struct.def", op.to_string()))
        }
    }
}

//===----------------------------------------------------------------------===//
// FieldDefOpLike
//===----------------------------------------------------------------------===//

/// Defines the public API of the 'struct.field' op.
pub trait FieldDefOpLike<'c: 'a, 'a>: OperationLike<'c, 'a> {
    fn has_public_attr(&self) -> bool {
        unsafe { llzkFieldDefOpGetHasPublicAttr(self.to_raw()) }
    }

    fn set_public_attr(&self, value: bool) {
        unsafe {
            llzkFieldDefOpSetPublicAttr(self.to_raw(), value);
        }
    }
}

//===----------------------------------------------------------------------===//
// FieldDefOp
//===----------------------------------------------------------------------===//

/// Represents an owned 'struct.field' op.
pub struct FieldDefOp<'c> {
    inner: Operation<'c>,
}

impl FieldDefOp<'_> {
    pub unsafe fn from_raw(raw: MlirOperation) -> Self {
        unsafe {
            Self {
                inner: Operation::from_raw(raw),
            }
        }
    }
}

impl<'a, 'c: 'a> OperationLike<'c, 'a> for FieldDefOp<'c> {
    fn to_raw(&self) -> MlirOperation {
        self.inner.to_raw()
    }
}

impl<'a, 'c: 'a> FieldDefOpLike<'c, 'a> for FieldDefOp<'c> {}

impl<'c> fmt::Display for FieldDefOp<'c> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'c> Deref for FieldDefOp<'c> {
    type Target = Operation<'c>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'c> Into<Operation<'c>> for FieldDefOp<'c> {
    fn into(self) -> Operation<'c> {
        self.inner
    }
}

impl<'c> TryFrom<Operation<'c>> for FieldDefOp<'c> {
    type Error = Error;

    fn try_from(op: Operation<'c>) -> Result<Self, Self::Error> {
        if unsafe { llzkOperationIsAFieldDefOp(op.to_raw()) } {
            Ok(unsafe { Self::from_raw(op.to_raw()) })
        } else {
            Err(Self::Error::OperationExpected(
                "struct.field",
                op.to_string(),
            ))
        }
    }
}

//===----------------------------------------------------------------------===//
// FieldDefOpRef
//===----------------------------------------------------------------------===//

/// Represents a non-owned reference to a 'struct.field' op.
#[derive(Clone, Copy)]
pub struct FieldDefOpRef<'c, 'a> {
    inner: OperationRef<'c, 'a>,
}

impl FieldDefOpRef<'_, '_> {
    pub unsafe fn from_raw(raw: MlirOperation) -> Self {
        unsafe {
            Self {
                inner: OperationRef::from_raw(raw),
            }
        }
    }
}

impl<'a, 'c: 'a> OperationLike<'c, 'a> for FieldDefOpRef<'c, 'a> {
    fn to_raw(&self) -> MlirOperation {
        self.inner.to_raw()
    }
}

impl<'a, 'c: 'a> FieldDefOpLike<'c, 'a> for FieldDefOpRef<'c, 'a> {}

impl fmt::Display for FieldDefOpRef<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl<'a, 'c: 'a> Deref for FieldDefOpRef<'c, 'a> {
    type Target = OperationRef<'c, 'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, 'c: 'a> Into<OperationRef<'c, 'a>> for FieldDefOpRef<'c, 'a> {
    fn into(self) -> OperationRef<'c, 'a> {
        self.inner
    }
}

impl<'a, 'c: 'a> TryFrom<OperationRef<'c, 'a>> for FieldDefOpRef<'c, 'a> {
    type Error = Error;

    fn try_from(op: OperationRef<'c, 'a>) -> Result<Self, Self::Error> {
        if unsafe { llzkOperationIsAFieldDefOp(op.to_raw()) } {
            Ok(unsafe { Self::from_raw(op.to_raw()) })
        } else {
            Err(Self::Error::OperationExpected(
                "struct.field",
                op.to_string(),
            ))
        }
    }
}

//===----------------------------------------------------------------------===//
// operation factories
//===----------------------------------------------------------------------===//

/// Creates a 'struct.def' op
pub fn def<'c>(
    location: Location<'c>,
    name: FlatSymbolRefAttribute<'c>,
    params: &[FlatSymbolRefAttribute<'c>],
) -> StructDefOp<'c> {
    let ctx = location.context();
    let params: Vec<Attribute> = params.into_iter().map(|a| (*a).into()).collect();
    let params = ArrayAttribute::new(unsafe { ctx.to_ref() }, &params).into();
    OperationBuilder::new("struct.def", location)
        .add_attributes(&[
            (ident!(ctx, "sym_name"), name.into()),
            (ident!(ctx, "const_param"), params),
        ])
        .add_regions([Region::new()])
        .build()
        .expect("valid operation")
        .try_into()
        .expect("operation of type 'struct.def'")
}

/// Creates a 'struct.field' op
pub fn field<'c>(
    location: Location<'c>,
    name: FlatSymbolRefAttribute<'c>,
    r#type: Type<'c>,
    is_column: bool,
    is_public: bool,
) -> FieldDefOp<'c> {
    let ctx = location.context();
    let r#type = TypeAttribute::new(r#type);
    let mut builder = OperationBuilder::new("struct.field", location).add_attributes(&[
        (ident!(ctx, "sym_name"), name.into()),
        (ident!(ctx, "type"), r#type.into()),
    ]);

    builder = if is_column {
        builder.add_attributes(&[(
            ident!(ctx, "column"),
            Attribute::unit(unsafe { ctx.to_ref() }),
        )])
    } else {
        builder
    };

    let op: FieldDefOp = builder
        .build()
        .expect("valid operation")
        .try_into()
        .expect("operation of type 'struct.field'");
    op.set_public_attr(is_public);
    op
}

/// Creates a 'struct.readf' op
pub fn readf<'c>(
    builder: &OpBuilder<'c>,
    location: Location<'c>,
    result_type: Type<'c>,
    component: Value<'c, '_>,
    field_name: &str,
) -> Operation<'c> {
    let field_name = StringRef::new(field_name);
    unsafe {
        Operation::from_raw(llzkFieldReadOpBuild(
            builder.to_raw(),
            location.to_raw(),
            result_type.to_raw(),
            component.to_raw(),
            field_name.to_raw(),
        ))
    }
}

/// Creates a 'struct.readf' op
pub fn readf_with_offset<'c>() -> Operation<'c> {
    todo!()
}

/// Creates a 'struct.writef' op
pub fn writef<'c>() -> Operation<'c> {
    todo!()
}

/// Creates a 'struct.new' op
pub fn new<'c>(location: Location<'c>, r#type: StructType<'c>) -> Operation<'c> {
    OperationBuilder::new("struct.new", location)
        .add_results(&[r#type.into()])
        .build()
        .expect("valid operation")
}
