use std::{borrow::Cow, marker::PhantomData, rc::Rc};

use anyhow::{anyhow, Result};
use llzk::dialect::r#struct::StructDefOp;
use llzk::{
    builder::OpBuilder,
    dialect::{
        constrain,
        felt::{self, FeltConstAttribute, FeltType, Radix},
        function::{FuncDefOpLike as _, FuncDefOpRef},
        r#struct::{self, FieldDefOpLike as _, FieldDefOpRef, StructDefOpLike},
    },
};
use melior::ir::ValueLike;
use melior::{
    ir::{
        attribute::FlatSymbolRefAttribute, operation::OperationLike as _, BlockLike as _, Location,
        Operation, OperationRef, RegionLike as _, Type, Value,
    },
    Context,
};
use midnight_halo2_proofs::plonk::{AdviceQuery, Challenge, FixedQuery, InstanceQuery, Selector};
use mlir_sys::MlirValue;
use num_bigint::BigUint;

use crate::backend::func::FieldId;
use crate::{
    backend::{
        func::{ArgNo, FuncIO},
        lowering::Lowering,
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    halo2::PrimeField,
    synthesis::regions::FQN,
    BinaryBoolOp,
};

use super::counter::Counter;
use super::extras::{block_list, operations_list};

pub struct LlzkStructLowering<'c, F> {
    struct_op: StructDefOp<'c>,
    constraints_counter: Rc<Counter>,
    _marker: PhantomData<F>,
}

impl<'c, F: PrimeField> LlzkStructLowering<'c, F> {
    pub fn new(struct_op: StructDefOp<'c>) -> Self {
        Self {
            struct_op,
            constraints_counter: Rc::new(Default::default()),
            _marker: Default::default(),
        }
    }

    pub fn take_struct(self) -> StructDefOp<'c> {
        self.struct_op
    }

    fn context(&self) -> &'c Context {
        unsafe { self.struct_op.context().to_ref() }
    }

    fn struct_name(&self) -> &str {
        StructDefOpLike::name(&self.struct_op)
    }

    /// Tries to fetch an advice cell field, if it doesn't exist creates a field that represents
    /// it.
    fn get_temp_decl(
        &self,
        col: usize,
        row: usize,
        fqn: Option<&Cow<FQN>>,
    ) -> Result<FieldDefOpRef<'c, '_>> {
        let name = format!("adv_{col}_{row}");
        Ok(self.struct_op.get_or_create_field_def(&name, || {
            let field_name = FlatSymbolRefAttribute::new(self.context(), &name);
            let filename = format!(
                "struct {} | advice field{}",
                self.struct_name(),
                fqn.map(|fqn| format!(" | {fqn}")).unwrap_or_default()
            );
            let loc = Location::new(self.context(), &filename, col, row);

            r#struct::field(loc, field_name, FeltType::new(self.context()), false, false)
        })?)
    }

    fn get_output(&self, field: FieldId) -> Result<FieldDefOpRef<'c, '_>> {
        self.struct_op
            .get_field_def(format!("out_{field}").as_str())
            .ok_or_else(|| anyhow!("Struct is missing output #{field}"))
    }

    fn get_constrain_func(&self) -> Result<FuncDefOpRef<'c, '_>> {
        self.struct_op
            .get_constrain_func()
            .ok_or_else(|| anyhow!("Constrain function is missing!"))
    }

    /// Adds an operation at the end of the constrain function.
    fn append_op<O>(&self, op: O) -> Result<OperationRef<'c, '_>>
    where
        O: Into<Operation<'c>>,
    {
        let block = self
            .get_constrain_func()?
            .region(0)?
            .first_block()
            .ok_or_else(|| anyhow!("Constraint function region is missing a block"))?;
        Ok(block.append_operation(op.into()))
    }

    /// Adds an operation at the end of the constrain function and returns the first resulf of the
    /// operation.
    fn append_expr<O>(&self, op: O) -> Result<Value<'c, '_>>
    where
        O: Into<Operation<'c>>,
    {
        Ok(self.append_op(op)?.result(0)?.into())
    }

    fn get_arg_impl(&self, idx: usize) -> Result<Value<'c, '_>> {
        self.get_constrain_func()?
            .argument(idx)
            .map(Into::into)
            .map_err(Into::into)
    }

    /// Returns the (n+1)-th argument of the constrain function. The index is offset by one because
    /// in the constrain function the first argument is always an instance of the struct.
    fn get_arg(&self, arg_no: ArgNo) -> Result<Value<'c, '_>> {
        self.get_arg_impl(*arg_no + 1)
    }

    fn get_component(&self) -> Result<Value<'c, '_>> {
        self.get_arg_impl(0)
    }

    fn read_field(&self, name: &str, result_type: Type<'c>) -> Result<Value<'c, '_>> {
        let builder = OpBuilder::new(self.context());

        self.append_expr(r#struct::readf(
            &builder,
            Location::unknown(self.context()),
            result_type,
            self.get_component()?,
            name,
        )?)
    }

    fn lower_constant_impl(&self, f: F) -> Result<Value<'c, '_>> {
        let repr = BigUint::from_bytes_le(f.to_repr().as_ref());
        let const_attr = FeltConstAttribute::parse(
            self.context(),
            repr.to_string().as_str(),
            repr.bits().try_into()?,
            Radix::Base10,
        );
        self.append_expr(felt::constant(
            Location::unknown(self.context()),
            const_attr,
        )?)
    }

    fn lower_resolved_query(
        &self,
        query: ResolvedQuery<F>,
        fqn: Option<&Cow<FQN>>,
    ) -> Result<Value<'c, '_>> {
        match query {
            ResolvedQuery::Lit(f) => self.lower_constant_impl(f),
            ResolvedQuery::IO(FuncIO::Arg(arg)) => self.get_arg(arg),
            ResolvedQuery::IO(FuncIO::Field(field)) => {
                let field = self.get_output(field)?;
                self.read_field(field.field_name(), field.field_type())
            }
            ResolvedQuery::IO(FuncIO::Advice(col, row)) => {
                let field = self.get_temp_decl(col, row, fqn)?;
                self.read_field(field.field_name(), field.field_type())
            }
            ResolvedQuery::IO(FuncIO::Fixed(_, _)) => todo!(),
            ResolvedQuery::IO(FuncIO::TableLookup(_, _, _, _, _)) => todo!(),
        }
    }
}

impl<'c, F: PrimeField> Lowering for LlzkStructLowering<'c, F> {
    //type CellOutput = Value<'c, '_>;
    type CellOutput = MlirValue;

    type F = F;

    fn generate_constraint(
        &self,
        op: BinaryBoolOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()> {
        let loc = Location::new(
            self.context(),
            format!("struct {} | constraints", self.struct_name()).as_str(),
            self.constraints_counter.next(),
            0,
        );
        self.append_op(match op {
            BinaryBoolOp::Eq => constrain::eq(loc, unsafe { Value::from_raw(*lhs) }, unsafe {
                Value::from_raw(*rhs)
            }),
            BinaryBoolOp::Lt => todo!(),
            BinaryBoolOp::Le => todo!(),
            BinaryBoolOp::Gt => todo!(),
            BinaryBoolOp::Ge => todo!(),
            BinaryBoolOp::Ne => todo!(),
        })?;
        Ok(())
    }

    fn num_constraints(&self) -> usize {
        self.get_constrain_func()
            .map(|op| {
                op.regions()
                    .flat_map(block_list)
                    .flat_map(operations_list)
                    .filter(|o| {
                        o.name()
                            .as_string_ref()
                            .as_str()
                            .map(|op_name| matches!(op_name, "constrain.eq"))
                            .unwrap_or_default()
                    })
                    .count()
            })
            .unwrap_or_default()
    }

    fn generate_comment(&self, _s: String) -> Result<()> {
        // If the final target is picus generate a 'picus.comment' op. Otherwise do nothing.
        unimplemented!()
    }

    fn generate_call(
        &self,
        _name: &str,
        _selectors: &[Self::CellOutput],
        _queries: &[FuncIO],
    ) -> Result<()> {
        // 1. Define a field of the type of the struct that is going to be called
        // 2. Load the field into a value
        // 3. Call the constrain function
        // 4. Read each output field from the struct into a ssa value
        unimplemented!()
    }

    fn lower_sum(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        self.append_expr(felt::add(
            Location::unknown(self.context()),
            unsafe { Value::from_raw(*lhs) },
            unsafe { Value::from_raw(*rhs) },
        )?)
        .map(|v| v.to_raw())
    }

    fn lower_product(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        self.append_expr(felt::mul(
            Location::unknown(self.context()),
            unsafe { Value::from_raw(*lhs) },
            unsafe { Value::from_raw(*rhs) },
        )?)
        .map(|v| v.to_raw())
    }

    fn lower_neg(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput> {
        self.append_expr(felt::neg(Location::unknown(self.context()), unsafe {
            Value::from_raw(*expr)
        })?)
        .map(|v| v.to_raw())
    }

    fn lower_scaled(
        &self,
        expr: &Self::CellOutput,
        scale: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        self.append_expr(felt::mul(
            Location::unknown(self.context()),
            unsafe { Value::from_raw(*expr) },
            unsafe { Value::from_raw(*scale) },
        )?)
        .map(|v| v.to_raw())
    }

    fn lower_challenge(&self, _challenge: &Challenge) -> Result<Self::CellOutput> {
        unimplemented!()
    }

    fn lower_selector(
        &self,
        sel: &Selector,
        resolver: &dyn SelectorResolver,
    ) -> Result<Self::CellOutput> {
        match resolver.resolve_selector(sel)? {
            ResolvedSelector::Const(b) => self.lower_constant(b.to_f()),
            ResolvedSelector::Arg(arg_no) => self.get_arg(arg_no).map(|v| v.to_raw()),
        }
    }

    fn lower_advice_query(
        &self,
        query: &AdviceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput> {
        let (query, fqn) = resolver.resolve_advice_query(query)?;
        self.lower_resolved_query(query, fqn.as_ref())
            .map(|v| v.to_raw())
    }

    fn lower_instance_query(
        &self,
        query: &InstanceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput> {
        self.lower_resolved_query(resolver.resolve_instance_query(query)?, None)
            .map(|v| v.to_raw())
    }

    fn lower_fixed_query(
        &self,
        query: &FixedQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput> {
        self.lower_resolved_query(resolver.resolve_fixed_query(query)?, None)
            .map(|v| v.to_raw())
    }

    fn lower_constant(&self, f: Self::F) -> Result<Self::CellOutput> {
        self.lower_constant_impl(f).map(|v| v.to_raw())
    }

    fn generate_assume_deterministic(&self, _func_io: FuncIO) -> Result<()> {
        // If the final target is picus generate a 'picus.assume_deterministic' op. Otherwise do nothing.
        todo!()
    }

    fn lower_eq(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_and(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_or(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        todo!()
    }

    fn generate_assert(&self, expr: &Self::CellOutput) -> Result<()> {
        todo!()
    }

    fn lower_function_input(&self, i: usize) -> FuncIO {
        todo!()
    }

    fn lower_function_output(&self, o: usize) -> FuncIO {
        todo!()
    }

    fn lower_funcio<IO>(&self, io: IO) -> Result<Self::CellOutput>
    where
        IO: Into<FuncIO>,
    {
        todo!()
    }
}
