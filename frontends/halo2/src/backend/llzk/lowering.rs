use std::rc::Rc;

use crate::backend::lowering::ExprLowering;
use crate::halo2::Challenge;
use anyhow::{anyhow, Result};
use llzk::builder::OpBuilder;
use llzk::prelude::*;
use melior::ir::ValueLike;
use melior::{
    ir::{
        attribute::FlatSymbolRefAttribute, operation::OperationLike as _, BlockLike as _, Location,
        Operation, OperationRef, RegionLike as _, Type, Value,
    },
    Context,
};
use mlir_sys::MlirValue;

use crate::backend::func::FieldId;
use crate::backend::{
    func::{ArgNo, FuncIO},
    lowering::Lowering,
};
use crate::ir::expr::Felt;
use crate::ir::CmpOp;

use super::counter::Counter;
use super::extras::{block_list, operations_list};

pub struct LlzkStructLowering<'c> {
    context: &'c Context,
    struct_op: StructDefOp<'c>,
    constraints_counter: Rc<Counter>,
}

impl<'c> LlzkStructLowering<'c> {
    pub fn new(context: &'c Context, struct_op: StructDefOp<'c>) -> Self {
        Self {
            context,
            struct_op,
            constraints_counter: Rc::new(Default::default()),
        }
    }

    pub fn take_struct(self) -> StructDefOp<'c> {
        self.struct_op
    }

    fn context(&self) -> &'c Context {
        self.context
    }

    fn struct_name(&self) -> &str {
        StructDefOpLike::name(&self.struct_op)
    }

    /// Tries to fetch an advice cell field, if it doesn't exist creates a field that represents
    /// it.
    #[allow(dead_code)]
    fn get_temp_decl(&self, col: usize, row: usize) -> Result<FieldDefOpRef<'c, '_>> {
        let name = format!("adv_{col}_{row}");
        Ok(self.struct_op.get_or_create_field_def(&name, || {
            let filename = format!("struct {} | advice field", self.struct_name(),);
            let loc = Location::new(self.context(), &filename, col, row);

            r#struct::field(loc, &name, FeltType::new(self.context()), false, false)
        })?)
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn get_arg_impl(&self, idx: usize) -> Result<Value<'c, '_>> {
        self.get_constrain_func()?
            .argument(idx)
            .map(Into::into)
            .map_err(Into::into)
    }

    /// Returns the (n+1)-th argument of the constrain function. The index is offset by one because
    /// in the constrain function the first argument is always an instance of the struct.
    #[allow(dead_code)]
    fn get_arg(&self, arg_no: ArgNo) -> Result<Value<'c, '_>> {
        self.get_arg_impl(*arg_no + 1)
    }

    #[allow(dead_code)]
    fn get_component(&self) -> Result<Value<'c, '_>> {
        self.get_arg_impl(0)
    }

    #[allow(dead_code)]
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

    fn lower_constant_impl(&self, f: Felt) -> Result<Value<'c, '_>> {
        let repr = f.as_ref();
        log::debug!("f as repr: {repr}");
        let const_attr = FeltConstAttribute::parse(self.context(), repr.to_string().as_str());
        self.append_expr(felt::constant(
            Location::unknown(self.context()),
            const_attr,
        )?)
    }
}

/// Value wrapper used as lowering output for circumventing lifetime restrictions.
#[derive(Copy, Clone, Debug)]
pub struct ValueWrap(MlirValue);

impl From<ValueWrap> for Value<'_, '_> {
    fn from(value: ValueWrap) -> Self {
        unsafe { Self::from_raw(value.0) }
    }
}

impl From<&ValueWrap> for Value<'_, '_> {
    fn from(value: &ValueWrap) -> Self {
        unsafe { Self::from_raw(value.0) }
    }
}

macro_rules! wrap {
    ($r:expr) => {
        ($r).map(|v| ValueWrap(v.to_raw()))
    };
}

impl<'c> Lowering for LlzkStructLowering<'c> {
    fn generate_constraint(
        &self,
        op: CmpOp,
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
            CmpOp::Eq => constrain::eq(loc, lhs.into(), rhs.into()),
            CmpOp::Lt => todo!(),
            CmpOp::Le => todo!(),
            CmpOp::Gt => todo!(),
            CmpOp::Ge => todo!(),
            CmpOp::Ne => todo!(),
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

    fn generate_comment(&self, s: String) -> Result<()> {
        // If the final target is picus generate a 'picus.comment' op. Otherwise do nothing.
        log::warn!("Comment {s:?} was not generated");
        Ok(())
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

    fn generate_assume_deterministic(&self, _func_io: FuncIO) -> Result<()> {
        // If the final target is picus generate a 'picus.assume_deterministic' op. Otherwise do nothing.
        todo!()
    }

    fn generate_assert(&self, _expr: &Self::CellOutput) -> Result<()> {
        todo!()
    }
}

impl<'c> ExprLowering for LlzkStructLowering<'c> {
    type CellOutput = ValueWrap;

    fn lower_sum(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        wrap! {
            self.append_expr(felt::add(
            Location::unknown(self.context()),
            lhs.into(),
            rhs.into(),
        )?)
        }
    }

    fn lower_product(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        wrap! {
            self.append_expr(felt::mul(
                Location::unknown(self.context()),
                lhs.into(),
                rhs.into(),
            )?)
        }
    }

    fn lower_neg(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput> {
        wrap! { self.append_expr(felt::neg(Location::unknown(self.context()), expr.into())?) }
    }

    fn lower_challenge(&self, _challenge: &Challenge) -> Result<Self::CellOutput> {
        unimplemented!()
    }

    fn lower_constant(&self, f: Felt) -> Result<Self::CellOutput> {
        wrap! {self.lower_constant_impl(f)}
    }

    fn lower_eq(
        &self,
        _lhs: &Self::CellOutput,
        _rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_and(
        &self,
        _lhs: &Self::CellOutput,
        _rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_or(
        &self,
        _lhs: &Self::CellOutput,
        _rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_function_input(&self, _i: usize) -> FuncIO {
        todo!()
    }

    fn lower_function_output(&self, _o: usize) -> FuncIO {
        todo!()
    }

    fn lower_funcio<IO>(&self, _io: IO) -> Result<Self::CellOutput>
    where
        IO: Into<FuncIO>,
    {
        todo!()
    }

    fn lower_lt(
        &self,
        _lhs: &Self::CellOutput,
        _rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_le(
        &self,
        _lhs: &Self::CellOutput,
        _rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_gt(
        &self,
        _lhs: &Self::CellOutput,
        _rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_ge(
        &self,
        _lhs: &Self::CellOutput,
        _rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_ne(
        &self,
        _lhs: &Self::CellOutput,
        _rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_not(&self, _value: &Self::CellOutput) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_true(&self) -> Result<Self::CellOutput> {
        todo!()
    }

    fn lower_false(&self) -> Result<Self::CellOutput> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
