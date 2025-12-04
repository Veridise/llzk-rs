use llzk_sys::{
    llzkFieldDefOpGetHasPublicAttr, llzkFieldDefOpSetPublicAttr, llzkFieldReadOpBuild,
    llzkOperationIsAFieldDefOp, llzkOperationIsAStructDefOp, llzkStructDefOpGetBody,
    llzkStructDefOpGetBodyRegion, llzkStructDefOpGetComputeFuncOp,
    llzkStructDefOpGetConstrainFuncOp, llzkStructDefOpGetFieldDef, llzkStructDefOpGetFieldDefs,
    llzkStructDefOpGetHasColumns, llzkStructDefOpGetHasParamName,
    llzkStructDefOpGetIsMainComponent, llzkStructDefOpGetNumFieldDefs, llzkStructDefOpGetType,
    llzkStructDefOpGetTypeWithParams,
};
use melior::{
    StringRef,
    ir::{
        Attribute, AttributeLike, Block, BlockLike as _, BlockRef, Identifier, Location, Operation,
        OperationRef, Region, RegionLike as _, RegionRef, Type, TypeLike, Value, ValueLike,
        attribute::{ArrayAttribute, FlatSymbolRefAttribute, StringAttribute, TypeAttribute},
        operation::{OperationBuilder, OperationLike, OperationMutLike},
    },
};
use mlir_sys::MlirOperation;

use crate::{
    builder::{OpBuilder, OpBuilderLike},
    dialect::function::FuncDefOpRef,
    error::Error,
    ident,
    macros::llzk_op_type,
};

use super::StructType;

//===----------------------------------------------------------------------===//
// StructDefOpLike
//===----------------------------------------------------------------------===//

/// Defines the public API of the 'struct.def' op.
pub trait StructDefOpLike<'c: 'a, 'a>: OperationLike<'c, 'a> {
    /// Returns the associated StructType to this op using the const params defined by the op.
    ///
    /// # Panics
    ///
    /// If the 'struct.def' op type is not `!struct.type`.
    fn r#type(&self) -> StructType<'c> {
        unsafe { Type::from_raw(llzkStructDefOpGetType(self.to_raw())) }
            .try_into()
            .expect("StructDefOpLike::type error")
    }

    /// Returns the name of the struct
    ///
    /// # Panics
    ///
    /// If the 'struct.def' op doesn't have an attribute named `sym_name`.
    fn name(&'a self) -> &'c str {
        self.attribute("sym_name")
            .and_then(StringAttribute::try_from)
            .map(|a| a.value())
            .unwrap()
    }

    /// Returns the single body Region of the StructDefOp.
    fn body_region(&self) -> RegionRef<'c, 'a> {
        unsafe { RegionRef::from_raw(llzkStructDefOpGetBodyRegion(self.to_raw())) }
    }

    /// Returns the single body Block within the StructDefOp's Region.
    fn body(&self) -> BlockRef<'c, 'a> {
        unsafe { BlockRef::from_raw(llzkStructDefOpGetBody(self.to_raw())) }
    }

    /// Returns the associated StructType to this op using the given const params instead of the
    /// parameters defined by the op.
    ///
    /// # Panics
    ///
    /// If the 'struct.def' op type is not `!struct.type`.
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
    ///
    /// # Panics
    ///
    /// If the nested symbol operation with the given name is not a `struct.field`.
    fn get_field_def(&self, name: &str) -> Option<FieldDefOpRef<'c, 'a>> {
        let name = StringRef::new(name);
        let raw_op = unsafe { llzkStructDefOpGetFieldDef(self.to_raw(), name.to_raw()) };
        if raw_op.ptr.is_null() {
            return None;
        }
        Some(
            unsafe { OperationRef::from_raw(raw_op) }
                .try_into()
                .expect("op of type 'struct.field'"),
        )
    }

    /// Returns the operation that defines the field with the given name, creating a new operation
    /// if not present.
    fn get_or_create_field_def<F>(&self, name: &str, f: F) -> Result<FieldDefOpRef<'c, 'a>, Error>
    where
        F: FnOnce() -> Result<FieldDefOp<'c>, Error>,
    {
        match self.get_field_def(name) {
            Some(f) => Ok(f),
            None => {
                let op = f()?;
                let region = self.region(0)?;
                let block = region
                    .first_block()
                    .unwrap_or_else(|| region.append_block(Block::new(&[])));

                let field_ref = block.append_operation(op.into());

                Ok(field_ref.try_into()?)
            }
        }
    }

    /// Fills the given array with the FieldDefOp operations inside this struct.  
    ///
    /// # Panics
    ///
    /// If any of the result operations is not a `struct.field` op.
    fn get_field_defs(&self) -> Vec<FieldDefOpRef<'c, '_>> {
        let num_fields = unsafe { llzkStructDefOpGetNumFieldDefs(self.to_raw()) };
        let mut raw_ops: Vec<MlirOperation> = Vec::with_capacity(num_fields.try_into().unwrap());
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
    fn has_columns(&self) -> bool {
        unsafe { llzkStructDefOpGetHasColumns(self.to_raw()) }.value != 0
    }

    /// Returns a [`FuncDefOpRef`] reference to the operation that defines the witness computation
    /// of the struct.
    ///
    /// # Panics
    ///
    /// If the result operation is not a `function.def`.
    fn get_compute_func<'b>(&self) -> Option<FuncDefOpRef<'c, 'b>> {
        let raw_op = unsafe { llzkStructDefOpGetComputeFuncOp(self.to_raw()) };
        if raw_op.ptr.is_null() {
            return None;
        }
        Some(
            unsafe { OperationRef::from_raw(raw_op) }
                .try_into()
                .expect("op of type 'function.def'"),
        )
    }

    /// Returns a [`FuncDefOpRef`] reference to the operation that defines the constraints of the
    /// struct.
    ///
    /// # Panics
    ///
    /// If the result operation is not a `function.def`.
    fn get_constrain_func<'b>(&self) -> Option<FuncDefOpRef<'c, 'b>> {
        let raw_op = unsafe { llzkStructDefOpGetConstrainFuncOp(self.to_raw()) };
        if raw_op.ptr.is_null() {
            return None;
        }
        Some(
            unsafe { OperationRef::from_raw(raw_op) }
                .try_into()
                .expect("op of type 'function.def'"),
        )
    }

    /// Returns true if the struct has a parameter with the given name.
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

/// Defines the mutable public API of the 'struct.def' op.
pub trait StructDefOpMutLike<'c: 'a, 'a>:
    StructDefOpLike<'c, 'a> + OperationMutLike<'c, 'a>
{
}

//===----------------------------------------------------------------------===//
// StructDefOp, StructDefOpRef, and StructDefOpRefMut
//===----------------------------------------------------------------------===//

llzk_op_type!(StructDefOp, llzkOperationIsAStructDefOp, "struct.def");

impl<'a, 'c: 'a> StructDefOpLike<'c, 'a> for StructDefOp<'c> {}

impl<'a, 'c: 'a> StructDefOpLike<'c, 'a> for StructDefOpRef<'c, 'a> {}

impl<'a, 'c: 'a> StructDefOpLike<'c, 'a> for StructDefOpRefMut<'c, 'a> {}

impl<'a, 'c: 'a> StructDefOpMutLike<'c, 'a> for StructDefOp<'c> {}

impl<'a, 'c: 'a> StructDefOpMutLike<'c, 'a> for StructDefOpRefMut<'c, 'a> {}

//===----------------------------------------------------------------------===//
// FieldDefOpLike
//===----------------------------------------------------------------------===//

/// Defines the public API of the 'struct.field' op.
pub trait FieldDefOpLike<'c: 'a, 'a>: OperationLike<'c, 'a> {
    /// Returns true if the field op has a `llzk.pub` attribute.
    fn has_public_attr(&self) -> bool {
        unsafe { llzkFieldDefOpGetHasPublicAttr(self.to_raw()) }
    }

    /// Sets or unsets the `llzk.pub` attribute.
    fn set_public_attr(&self, value: bool) {
        unsafe {
            llzkFieldDefOpSetPublicAttr(self.to_raw(), value);
        }
    }

    /// Returns the name of the field.
    ///
    /// # Panics
    ///
    /// If the 'struct.field' op doesn't have an attribute named `sym_name`.
    fn field_name(&self) -> &'c str {
        self.attribute("sym_name")
            .and_then(StringAttribute::try_from)
            .expect("malformed 'struct.field' op")
            .value()
    }

    /// Returns the type of the field.
    ///
    /// # Panics
    ///
    /// If the 'struct.field' op doesn't have a attribute named `type`.
    fn field_type(&self) -> Type<'c> {
        self.attribute("type")
            .and_then(TypeAttribute::try_from)
            .expect("malformed 'struct.field' op")
            .value()
    }
}

//===----------------------------------------------------------------------===//
// FieldDefOp, FieldDefOpRef, FieldDefOpRefMut
//===----------------------------------------------------------------------===//

llzk_op_type!(FieldDefOp, llzkOperationIsAFieldDefOp, "struct.field");

impl<'a, 'c: 'a> FieldDefOpLike<'c, 'a> for FieldDefOp<'c> {}

impl<'a, 'c: 'a> FieldDefOpLike<'c, 'a> for FieldDefOpRef<'c, 'a> {}

impl<'a, 'c: 'a> FieldDefOpLike<'c, 'a> for FieldDefOpRefMut<'c, 'a> {}

//===----------------------------------------------------------------------===//
// Operation factories
//===----------------------------------------------------------------------===//

/// Creates a 'struct.def' op
pub fn def<'c, I>(
    location: Location<'c>,
    name: &str,
    params: &[&str],
    region_ops: I,
) -> Result<StructDefOp<'c>, Error>
where
    I: IntoIterator<Item = Result<Operation<'c>, Error>>,
{
    let ctx = location.context();
    let params: Vec<Attribute> = params
        .iter()
        .map(|a| FlatSymbolRefAttribute::new(unsafe { ctx.to_ref() }, a).into())
        .collect();
    let params = ArrayAttribute::new(unsafe { ctx.to_ref() }, &params).into();
    let region = Region::new();
    let block = Block::new(&[]);
    region_ops
        .into_iter()
        .try_for_each(|op| -> Result<(), Error> {
            block.append_operation(op?);
            Ok(())
        })?;
    region.append_block(block);
    let name: Attribute = StringAttribute::new(unsafe { ctx.to_ref() }, name).into();
    let attrs = [
        (Identifier::new(unsafe { ctx.to_ref() }, "sym_name"), name),
        (
            Identifier::new(unsafe { ctx.to_ref() }, "const_params"),
            params,
        ),
    ];

    OperationBuilder::new("struct.def", location)
        .add_attributes(&attrs)
        .add_regions([region])
        .build()
        .map_err(Into::into)
        .and_then(TryInto::try_into)
}

/// Creates a 'struct.field' op
pub fn field<'c, T>(
    location: Location<'c>,
    name: &str,
    r#type: T,
    is_column: bool,
    is_public: bool,
) -> Result<FieldDefOp<'c>, Error>
where
    T: Into<Type<'c>>,
{
    let ctx = location.context();
    let r#type = TypeAttribute::new(r#type.into());
    let mut builder = OperationBuilder::new("struct.field", location).add_attributes(&[
        (
            ident!(ctx, "sym_name"),
            StringAttribute::new(unsafe { ctx.to_ref() }, name).into(),
        ),
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

    builder
        .build()
        .map_err(Into::into)
        .and_then(TryInto::try_into)
        .inspect(|op: &FieldDefOp<'c>| op.set_public_attr(is_public))
}

/// Creates a 'struct.readf' op
pub fn readf<'c>(
    builder: &OpBuilder<'c>,
    location: Location<'c>,
    result_type: Type<'c>,
    component: Value<'c, '_>,
    field_name: &str,
) -> Result<Operation<'c>, Error> {
    let field_name = StringRef::new(field_name);
    unsafe {
        let raw = llzkFieldReadOpBuild(
            builder.to_raw(),
            location.to_raw(),
            result_type.to_raw(),
            component.to_raw(),
            field_name.to_raw(),
        );
        if raw.ptr.is_null() {
            Err(Error::BuildMethodFailed("readf"))
        } else {
            Ok(Operation::from_raw(raw))
        }
    }
}

/// Creates a 'struct.readf' op.
///
/// This factory method is not implemented yet.
pub fn readf_with_offset<'c>() -> Operation<'c> {
    todo!()
}

/// Creates a 'struct.writef' op.
pub fn writef<'c>(
    location: Location<'c>,
    component: Value<'c, '_>,
    field_name: &str,
    value: Value<'c, '_>,
) -> Result<Operation<'c>, Error> {
    let context = location.context();
    let field_name = FlatSymbolRefAttribute::new(unsafe { context.to_ref() }, field_name);
    let attrs = [(
        Identifier::new(unsafe { context.to_ref() }, "field_name"),
        field_name.into(),
    )];
    OperationBuilder::new("struct.writef", location)
        .add_operands(&[component, value])
        .add_attributes(&attrs)
        .build()
        .map_err(Into::into)
}

/// Creates a 'struct.new' op
pub fn new<'c>(location: Location<'c>, r#type: StructType<'c>) -> Operation<'c> {
    OperationBuilder::new("struct.new", location)
        .add_results(&[r#type.into()])
        .build()
        .expect("valid operation")
}
