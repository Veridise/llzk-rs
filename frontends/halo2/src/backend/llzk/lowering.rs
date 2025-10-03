use std::rc::Rc;

use crate::backend::lowering::ExprLowering;
use crate::halo2::Challenge;
use anyhow::{anyhow, Result};
use llzk::builder::OpBuilder;
use llzk::prelude::*;
use melior::dialect::arith;
use melior::ir::attribute::IntegerAttribute;
use melior::ir::r#type::IntegerType;
use melior::ir::ValueLike;
use melior::{
    ir::{
        operation::OperationLike as _, BlockLike as _, Location,
        Operation, OperationRef, RegionLike as _, Value,
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

pub struct LlzkStructLowering<'c, 's> {
    context: &'c Context,
    struct_op: StructDefOpRefMut<'c, 's>,
    constraints_counter: Rc<Counter>,
}

impl<'c, 's> LlzkStructLowering<'c, 's> {
    pub fn new(context: &'c Context, struct_op: StructDefOpRefMut<'c, 's>) -> Self {
        Self {
            context,
            struct_op,
            constraints_counter: Rc::new(Default::default()),
        }
    }

    fn context(&self) -> &'c Context {
        self.context
    }

    fn struct_name(&self) -> &str {
        StructDefOpLike::name(&self.struct_op)
    }

    fn get_cell_field(&self, kind: &str, col: usize, row: usize) -> Result<FieldDefOpRef<'c, '_>> {
        let name = format!("{kind}_{col}_{row}");
        Ok(self.struct_op.get_or_create_field_def(&name, || {
            let filename = format!("struct {} | advice cell", self.struct_name(),);
            let loc = Location::new(self.context(), &filename, col, row);

            r#struct::field(loc, &name, FeltType::new(self.context()), false, false)
        })?)
    }

    /// Tries to fetch an advice cell field, if it doesn't exist creates a field that represents
    /// it.
    #[inline]
    fn get_adv_cell(&self, col: usize, row: usize) -> Result<FieldDefOpRef<'c, '_>> {
        self.get_cell_field("adv", col, row)
    }

    /// Tries to fetch an advice cell field, if it doesn't exist creates a field that represents
    /// it.
    #[inline]
    fn get_fix_cell(&self, col: usize, row: usize) -> Result<FieldDefOpRef<'c, '_>> {
        self.get_cell_field("fix", col, row)
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
        let op_ref = block.insert_operation_before(
            block
                .terminator()
                .ok_or_else(|| anyhow!("Constraint function is missing a terminator"))?,
            op.into(),
        );
        log::debug!("Inserted operation {op_ref}");
        Ok(op_ref)
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
        Ok(self.get_constrain_func()?.argument(idx)?.into())
    }

    /// Returns the (n+1)-th argument of the constrain function. The index is offset by one because
    /// in the constrain function the first argument is always an instance of the struct.
    fn get_arg(&self, arg_no: ArgNo) -> Result<Value<'c, '_>> {
        let val = self.get_arg_impl(*arg_no + 1)?;
        let signal_typ = StructType::from_str(self.context(), "Signal");
        if val.r#type() == signal_typ.into() {
            let builder = OpBuilder::new(self.context());
            return self.append_expr(r#struct::readf(
                &builder,
                Location::unknown(self.context()),
                FeltType::new(self.context()).into(),
                val,
                "reg",
            )?);
        }
        Ok(val)
    }

    fn get_component(&self) -> Result<Value<'c, '_>> {
        self.get_arg_impl(0)
    }

    fn read_field(&self, field: FieldDefOpRef<'c, '_>) -> Result<Value<'c, '_>> {
        let builder = OpBuilder::new(self.context());

        self.append_expr(r#struct::readf(
            &builder,
            Location::unknown(self.context()),
            field.field_type(),
            self.get_component()?,
            field.field_name(),
        )?)
    }

    fn lower_constant_impl(&self, f: Felt) -> Result<Value<'c, '_>> {
        let const_attr = FeltConstAttribute::from_biguint(self.context(), f.as_ref());
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

impl<'c> Lowering for LlzkStructLowering<'c, '_> {
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
        let cond = match op {
            CmpOp::Eq => {
                self.append_op(constrain::eq(loc, lhs.into(), rhs.into()))?;
                return Ok(());
            }
            CmpOp::Lt => self.lower_lt(lhs, rhs),
            CmpOp::Le => self.lower_le(lhs, rhs),
            CmpOp::Gt => self.lower_gt(lhs, rhs),
            CmpOp::Ge => self.lower_ge(lhs, rhs),
            CmpOp::Ne => self.lower_ne(lhs, rhs),
        }?;
        self.generate_assert(&cond)
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
        _inputs: &[Self::CellOutput],
        _outputs: &[FuncIO],
    ) -> Result<()> {
        // 1. Define a field of the type of the struct that is going to be called
        // 2. Load the field into a value
        // 3. Call the constrain function
        todo!()
    }

    fn generate_assume_deterministic(&self, _func_io: FuncIO) -> Result<()> {
        // If the final target is picus generate a 'picus.assume_deterministic' op. Otherwise do nothing.
        todo!(
            "There isn't yet a construct in LLZK that supports the 'assume_deterministic' statement"
        )
    }

    fn generate_assert(&self, expr: &Self::CellOutput) -> Result<()> {
        self.append_op(bool::assert(
            Location::unknown(self.context()),
            expr.into(),
            None,
        )?)?;
        Ok(())
    }
}

impl<'c> ExprLowering for LlzkStructLowering<'c, '_> {
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

    fn lower_eq(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(bool::eq(
            Location::unknown(self.context()),
            lhs.into(),
            rhs.into()
        )?))
    }

    fn lower_and(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(bool::and(
            Location::unknown(self.context()),
            lhs.into(),
            rhs.into()
        )?))
    }

    fn lower_or(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(bool::or(
            Location::unknown(self.context()),
            lhs.into(),
            rhs.into()
        )?))
    }

    fn lower_function_input(&self, i: usize) -> FuncIO {
        ArgNo::from(i).into()
    }

    fn lower_function_output(&self, o: usize) -> FuncIO {
        FieldId::from(o).into()
    }

    fn lower_funcio<IO>(&self, io: IO) -> Result<Self::CellOutput>
    where
        IO: Into<FuncIO>,
    {
        match io.into() {
            FuncIO::Arg(arg_no) => wrap!(self.get_arg(arg_no)),
            FuncIO::Field(field_id) => wrap!(self.read_field(self.get_output(field_id)?)),
            FuncIO::Advice(cell) => {
                wrap!(self.read_field(self.get_adv_cell(cell.col(), cell.row())?))
            }
            FuncIO::Fixed(cell) => {
                wrap!(self.read_field(self.get_fix_cell(cell.col(), cell.row())?))
            }
            FuncIO::TableLookup(_, _, _, _, _) => todo!(),
            FuncIO::CallOutput(_, _) => todo!(),
        }
    }

    fn lower_lt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(bool::lt(
            Location::unknown(self.context()),
            lhs.into(),
            rhs.into()
        )?))
    }

    fn lower_le(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(bool::le(
            Location::unknown(self.context()),
            lhs.into(),
            rhs.into()
        )?))
    }

    fn lower_gt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(bool::gt(
            Location::unknown(self.context()),
            lhs.into(),
            rhs.into()
        )?))
    }

    fn lower_ge(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(bool::ge(
            Location::unknown(self.context()),
            lhs.into(),
            rhs.into()
        )?))
    }

    fn lower_ne(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(bool::ne(
            Location::unknown(self.context()),
            lhs.into(),
            rhs.into()
        )?))
    }

    fn lower_not(&self, value: &Self::CellOutput) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(bool::not(Location::unknown(self.context()), value.into(),)?))
    }

    fn lower_true(&self) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(arith::constant(
            self.context(),
            IntegerAttribute::new(IntegerType::new(self.context(), 1).into(), 1).into(),
            Location::unknown(self.context())
        )))
    }

    fn lower_false(&self) -> Result<Self::CellOutput> {
        wrap!(self.append_expr(arith::constant(
            self.context(),
            IntegerAttribute::new(IntegerType::new(self.context(), 1).into(), 0).into(),
            Location::unknown(self.context())
        )))
    }
}

#[cfg(test)]
mod tests {
    use ff::Field as _;
    use log::LevelFilter;
    use melior::ir::Module;
    use simplelog::{Config, TestLogger};

    use crate::{
        backend::{
            codegen::Codegen as _,
            llzk::{LlzkCodegen, LlzkCodegenState},
        },
        io::{AdviceIO, InstanceIO},
        LlzkParamsBuilder,
    };

    use super::*;

    use rstest::{fixture, rstest};

    #[fixture]
    fn fragment_main() -> FragmentCfg {
        FragmentCfg {
            struct_name: "Main",
            n_inputs: 2,
            n_public_inputs: 1,
            n_outputs: 2,
            n_public_outputs: 1,
            self_name: "self",
            advice_cells: vec![],
            fixed_cells: vec![],
            is_main: true,
        }
    }

    #[fixture]
    fn fragment_main_with_cells() -> FragmentCfg {
        FragmentCfg {
            struct_name: "Main",
            n_inputs: 2,
            n_public_inputs: 1,
            n_outputs: 2,
            n_public_outputs: 1,
            self_name: "self",
            advice_cells: vec![(1, 5)],
            fixed_cells: vec![(2, 3)],
            is_main: true,
        }
    }

    #[rstest]
    fn lower_reading_cells(fragment_main_with_cells: FragmentCfg) {
        fragment_test(
            fragment_main_with_cells,
            r"%0 = struct.readf %self[@adv_1_5] : <@Main<[]>>, !felt.type
              %1 = struct.readf %self[@fix_2_3] : <@Main<[]>>, !felt.type",
            |l| {
                l.lower_funcio(FuncIO::advice_abs(1, 5))?;
                l.lower_funcio(FuncIO::fixed_abs(2, 3))?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_sum(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = felt.add %0, %0",
            |l| {
                let arg = l.lower_funcio(l.lower_function_input(0))?;
                l.lower_sum(&arg, &arg)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_sum_with_io(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = struct.readf %arg2[@reg] : <@Signal<[]>>, !felt.type
              %2 = struct.readf %self[@out_0] : <@Main<[]>>, !felt.type
              %3 = struct.readf %self[@out_1] : <@Main<[]>>, !felt.type
              %4 = felt.add %0, %2
              %5 = felt.add %1, %3",
            |l| {
                let arg0 = l.lower_funcio(l.lower_function_input(0))?;
                let arg1 = l.lower_funcio(l.lower_function_input(1))?;
                let out0 = l.lower_funcio(l.lower_function_output(0))?;
                let out1 = l.lower_funcio(l.lower_function_output(1))?;
                l.lower_sum(&arg0, &out0)?;
                l.lower_sum(&arg1, &out1)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_product(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = felt.mul %0, %0",
            |l| {
                let arg = l.lower_funcio(l.lower_function_input(0))?;
                l.lower_product(&arg, &arg)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_neg(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = felt.neg %0",
            |l| {
                let arg = l.lower_funcio(l.lower_function_input(0))?;
                l.lower_neg(&arg)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_eq(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = bool.cmp eq(%0, %0)",
            |l| {
                let arg = l.lower_funcio(l.lower_function_input(0))?;
                l.lower_eq(&arg, &arg)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_lt(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = bool.cmp lt(%0, %0)",
            |l| {
                let arg = l.lower_funcio(l.lower_function_input(0))?;
                l.lower_lt(&arg, &arg)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_le(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = bool.cmp le(%0, %0)",
            |l| {
                let arg = l.lower_funcio(l.lower_function_input(0))?;
                l.lower_le(&arg, &arg)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_gt(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = bool.cmp gt(%0, %0)",
            |l| {
                let arg = l.lower_funcio(l.lower_function_input(0))?;
                l.lower_gt(&arg, &arg)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_ge(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = bool.cmp ge(%0, %0)",
            |l| {
                let arg = l.lower_funcio(l.lower_function_input(0))?;
                l.lower_ge(&arg, &arg)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_ne(fragment_main: FragmentCfg) {
        fragment_test(
            fragment_main,
            r"%0 = struct.readf %arg1[@reg] : <@Signal<[]>>, !felt.type
              %1 = bool.cmp ne(%0, %0)",
            |l| {
                let arg = l.lower_funcio(l.lower_function_input(0))?;
                l.lower_ne(&arg, &arg)?;
                Ok(())
            },
        )
    }

    #[rstest]
    #[should_panic]
    fn lower_and(fragment_main: FragmentCfg) {
        // bool.and is not allowed in the constrain function.
        fragment_test(
            fragment_main,
            r"%true = arith.constant true
              %1 = bool.and %true, %true",
            |l| {
                let t = l.lower_true()?;
                l.lower_and(&t, &t)?;
                Ok(())
            },
        )
    }

    #[rstest]
    #[should_panic]
    fn lower_or(fragment_main: FragmentCfg) {
        // bool.or is not allowed in the constrain function.
        fragment_test(
            fragment_main,
            r"%true = arith.constant true
              %1 = bool.or %true, %true",
            |l| {
                let t = l.lower_true()?;
                l.lower_or(&t, &t)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_true(fragment_main: FragmentCfg) {
        fragment_test(fragment_main, "%true = arith.constant true", |l| {
            l.lower_true()?;
            Ok(())
        })
    }

    #[rstest]
    fn lower_false(fragment_main: FragmentCfg) {
        fragment_test(fragment_main, "%false = arith.constant false", |l| {
            l.lower_false()?;
            Ok(())
        })
    }

    #[rstest]
    #[should_panic]
    fn lower_not(fragment_main: FragmentCfg) {
        // bool.not is not allowed in the constrain function.
        fragment_test(
            fragment_main,
            "%true = arith.constant true\n%0 = bool.not %true",
            |l| {
                let t = l.lower_true()?;
                l.lower_not(&t)?;
                Ok(())
            },
        )
    }

    #[rstest]
    fn lower_constant(fragment_main: FragmentCfg) {
        fragment_test(fragment_main, "%felt_const_1 = felt.const 1", |l| {
            // Loading a constant is not supported right now.
            l.lower_constant(Felt::new(crate::halo2::Fr::ONE))?;
            Ok(())
        })
    }

    /// Empty test to make sure the basic structure works as intended.
    #[rstest]
    fn empty_fragment(fragment_main: FragmentCfg) {
        fragment_test(fragment_main, "", |_| Ok(()))
    }

    /// Test infrastructure for testing the lowering module inside the correct context.
    ///
    /// Creates a codegen module and instantiates the lowering component inside a struct.
    /// The test is defined inside the closure, making calls to [`LlzkStructLowering`].
    /// The structs is then lowered whole into MLIR IR.
    ///
    /// The expected behavior is defined in textual MLIR IR as the fragment. This fragment is
    /// injected into a textual representation of the final module and compared against the emitted
    /// module. To avoid whitespacing issues or other formatting issues the textual IR is parsed
    /// into a [`Module`] and then reprinted to standarize the syntax.
    fn fragment_test(
        cfg: FragmentCfg,
        frag: &str,
        test: impl FnOnce(&LlzkStructLowering) -> Result<()>,
    ) {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        let context = LlzkContext::new();
        let state: LlzkCodegenState = LlzkParamsBuilder::new(&context)
            .with_top_level(cfg.struct_name)
            .build()
            .into();
        let codegen = LlzkCodegen::initialize(&state);
        let advice_io = cfg.advice_io();
        let instance_io = cfg.instance_io();
        let s = if cfg.is_main {
            codegen.define_main_function(&advice_io, &instance_io)
        } else {
            assert_eq!(cfg.n_public_inputs, 0);
            assert_eq!(cfg.n_public_outputs, 0);
            codegen.define_function(cfg.struct_name, cfg.n_inputs, cfg.n_outputs)
        }
        .unwrap();
        test(&s).unwrap();
        codegen.on_scope_end(s).unwrap();

        let out = codegen.generate_output().unwrap();
        assert!(
            out.module().as_operation().verify(),
            "Top level module failed verification"
        );

        let fragment = expected_fragment(&context, &cfg, frag);
        let out_str = format!("{}", out.module().as_operation());
        let frag_str = format!("{}", fragment.as_operation());
        similar_asserts::assert_eq!(out_str, frag_str)
    }

    struct FragmentCfg {
        struct_name: &'static str,
        n_inputs: usize,
        n_public_inputs: usize,
        n_outputs: usize,
        n_public_outputs: usize,
        self_name: &'static str,
        advice_cells: Vec<(usize, usize)>,
        fixed_cells: Vec<(usize, usize)>,
        is_main: bool,
    }

    impl FragmentCfg {
        fn advice_io(&self) -> AdviceIO {
            let mut cs = crate::halo2::ConstraintSystem::<crate::halo2::Fr>::default();
            let inputs = Vec::from_iter(self.n_public_inputs..self.n_inputs);
            let outputs = Vec::from_iter(self.n_public_outputs..self.n_outputs);
            AdviceIO::new(
                &[(cs.advice_column(), &inputs)],
                &[(cs.advice_column(), &outputs)],
            )
        }

        fn instance_io(&self) -> InstanceIO {
            let mut cs = crate::halo2::ConstraintSystem::<crate::halo2::Fr>::default();
            let inputs = Vec::from_iter(0..self.n_public_inputs);
            let outputs = Vec::from_iter(0..self.n_public_outputs);
            InstanceIO::new(
                &[(cs.instance_column(), &inputs)],
                &[(cs.instance_column(), &outputs)],
            )
        }

        fn inputs(&self) -> String {
            (1..=self.n_inputs)
                .map(|n| {
                    format!(
                        "{} %arg{n}: {}{}",
                        if n == 1 { "" } else { "," },
                        self.input_type_str(),
                        if n <= self.n_public_inputs {
                            " {llzk.pub = #llzk.pub}"
                        } else {
                            ""
                        }
                    )
                })
                .collect()
        }

        fn input_type_str(&self) -> &'static str {
            if self.is_main {
                "!struct.type<@Signal<[]>>"
            } else {
                "!felt.type"
            }
        }

        fn cells(&self) -> String {
            self.advice_cells
                .iter()
                .map(|(col, row)| format!("struct.field @adv_{col}_{row} : !felt.type\n"))
                .chain(
                    self.fixed_cells
                        .iter()
                        .map(|(col, row)| format!("struct.field @fix_{col}_{row} : !felt.type\n")),
                )
                .collect()
        }

        fn fields(&self) -> String {
            (0..self.n_outputs)
                .map(|n| {
                    format!(
                        "struct.field @out_{n} : !felt.type{}\n",
                        if n < self.n_public_outputs {
                            " {llzk.pub}"
                        } else {
                            ""
                        }
                    )
                })
                .collect()
        }

        fn signal(&self) -> &'static str {
            r#"
  struct.def @Signal<[]> {
    struct.field @reg : !felt.type {llzk.pub}
    function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@Signal<[]>>
      struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
      function.return %self : !struct.type<@Signal<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint} {
      function.return
    }
  }"#
        }
    }

    fn expected_fragment<'c>(ctx: &'c Context, cfg: &FragmentCfg, frag: &str) -> Module<'c> {
        let src = format!(
            r#"module attributes {{veridise.lang = "llzk"}} {{
  {signal}
  struct.def @{name}<[]> {{
    {fields}
    function.def @compute({inputs}) -> !struct.type<@{name}<[]>> attributes {{function.allow_witness}} {{
      %{self_name} = struct.new : <@{name}<[]>>
      function.return %{self_name} : !struct.type<@{name}<[]>>
    }}
    function.def @constrain(%{self_name}: !struct.type<@{name}<[]>>, {inputs}) attributes {{function.allow_constraint}} {{
      {frag}
      function.return
    }}
    {cells}
  }}
}}"#,
            name = cfg.struct_name,
            inputs = cfg.inputs(),
            fields = cfg.fields(),
            signal = cfg.signal(),
            self_name = cfg.self_name,
            cells = cfg.cells()
        );
        Module::parse(ctx, src.as_str()).unwrap_or_else(|| panic!("Failed to parse check: {src}"))
    }
}
