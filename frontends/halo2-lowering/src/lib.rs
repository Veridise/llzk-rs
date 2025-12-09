#![doc = include_str!("../README.md")]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

use haloumi_core::{cmp::CmpOp, felt::Felt, func::FuncIO};

pub mod error;
pub mod lowerable;

/// Result type for lowering related operations.
pub type Result<T> = std::result::Result<T, error::Error>;

/// Defines the interface code generators expose for generating code in their corresponding IR.
pub trait Lowering: ExprLowering {
    /// Generates a constraint.
    fn generate_constraint(
        &self,
        op: CmpOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()>;

    /// Returns the number of generated constraints.
    fn num_constraints(&self) -> usize;

    /// Attempts to generate a constraint and fails if it couldn't be generated.
    fn checked_generate_constraint(
        &self,
        op: CmpOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()> {
        let before = self.num_constraints();
        self.generate_constraint(op, lhs, rhs)?;
        let after = self.num_constraints();
        if before >= after {
            return Err(error::Error::LastConstraintNotGenerated);
        }
        Ok(())
    }

    /// Generates IR representing a comment with the given text.
    fn generate_comment(&self, s: String) -> Result<()>;

    /// Generates an statement that hints that the given [`FuncIO`] must be assumed to be
    /// deterministic.
    fn generate_assume_deterministic(&self, func_io: FuncIO) -> Result<()>;

    /// Generates a call to another group.
    fn generate_call(
        &self,
        name: &str,
        selectors: &[Self::CellOutput],
        outputs: &[FuncIO],
    ) -> Result<()>;

    /// Generates an assertion using the given expression.
    ///
    /// How exactly this assertion is represented is backend dependant.
    fn generate_assert(&self, expr: &Self::CellOutput) -> Result<()>;

    /// Generates an assertion using the given expression that is treated as a post-condition.
    ///
    /// How exactly this assertion is represented is backend dependant.
    fn generate_post_condition(&self, expr: &Self::CellOutput) -> Result<()>;
}

/// Defines the interface code generators expose for generating expressions in their corresponding IR.
pub trait ExprLowering {
    /// The type representing a generated expression.
    type CellOutput;

    /// Emits an expression representing addition.
    fn lower_sum(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput)
    -> Result<Self::CellOutput>;

    /// Emits an expression representing multiplication.
    fn lower_product(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput>;

    /// Emits an expression representing negation.
    fn lower_neg(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a constant value.
    fn lower_constant(&self, f: Felt) -> Result<Self::CellOutput>;

    /// Emits a boolean expression representing equality between the operands.
    fn lower_eq(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a boolean expression representing a less-than relation between the operands.
    fn lower_lt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a boolean expression representing a less-than or equal relation between the operands.
    fn lower_le(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a boolean expression representing a greater-than relation between the operands.
    fn lower_gt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a boolean expression representing a greater-than or equal relation between the operands.
    fn lower_ge(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a boolean expression representing the negation of equality between the operands.
    fn lower_ne(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a logical AND between the two operands.
    fn lower_and(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput)
    -> Result<Self::CellOutput>;

    /// Emits a logical OR between the two operands.
    fn lower_or(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a logical NOT between the two operands.
    fn lower_not(&self, value: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a literal `true` value.
    fn lower_true(&self) -> Result<Self::CellOutput>;

    /// Emits a literal `false` value.
    fn lower_false(&self) -> Result<Self::CellOutput>;

    /// Emits an expression that hints that the given expression must be proven deterministic.
    ///
    /// The concrete semantics of this expression are backend dependant but it must return an
    /// expression of boolean type.
    fn lower_det(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput>;

    /// Emits a logical implication between the two operands.
    fn lower_implies(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput>;

    /// Emits a logical double-implication between the two operands.
    fn lower_iff(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput)
    -> Result<Self::CellOutput>;

    /// Returns a [`FuncIO`] representing the `i`-th input.
    fn lower_function_input(&self, i: usize) -> FuncIO;

    /// Returns a [`FuncIO`] representing the `o`-th output.
    fn lower_function_output(&self, o: usize) -> FuncIO;

    /// Returns a list of [`FuncIO`] representing the input indices in the iterator.
    fn lower_function_inputs(&self, ins: impl IntoIterator<Item = usize>) -> Vec<FuncIO> {
        ins.into_iter()
            .map(|i| self.lower_function_input(i))
            .collect()
    }

    /// Returns a list of [`FuncIO`] representing the output indices in the iterator.
    fn lower_function_outputs(&self, outs: impl IntoIterator<Item = usize>) -> Vec<FuncIO> {
        outs.into_iter()
            .map(|o| self.lower_function_output(o))
            .collect()
    }

    /// Emits an expression representing the given IO.
    fn lower_funcio<IO>(&self, io: IO) -> Result<Self::CellOutput>
    where
        IO: Into<FuncIO>;
}
