use backend::MockExprIR;

use crate::halo2::Fr;
pub mod backend;

#[derive(Default)]
pub struct IRBuilder {
    ir: Vec<MockExprIR>,
    stack: Vec<usize>,
}

impl IRBuilder {
    fn push(&mut self) {
        self.stack.push(self.ir.len());
    }

    fn pop(&mut self) -> usize {
        self.stack.pop().unwrap()
    }

    pub fn push_const(mut self, f: Fr) -> Self {
        self.push();
        self.ir.push(MockExprIR::Const(f));
        self
    }

    pub fn push_temp(mut self, col: usize, row: usize) -> Self {
        self.push();
        self.ir.push(MockExprIR::Temp(col, row));
        self
    }

    pub fn push_fixed(mut self, col: usize, row: usize) -> Self {
        self.push();
        self.ir.push(MockExprIR::Fixed(col, row));
        self
    }

    pub fn push_arg(mut self, idx: usize) -> Self {
        self.push();
        self.ir.push(MockExprIR::Arg(idx.into()));
        self
    }

    pub fn push_field(mut self, idx: usize) -> Self {
        self.push();
        self.ir.push(MockExprIR::Field(idx.into()));
        self
    }

    fn pop_bin_impl(
        &mut self,
        lhs: Option<usize>,
        rhs: Option<usize>,
        f: fn(usize, usize) -> MockExprIR,
    ) {
        let rhs = rhs.unwrap_or_else(|| self.pop());
        let lhs = lhs.unwrap_or_else(|| self.pop());
        self.push();
        self.ir.push(f(lhs, rhs));
    }

    fn pop_un_impl(&mut self, idx: Option<usize>, f: fn(usize) -> MockExprIR) {
        let idx = idx.unwrap_or_else(|| self.pop());
        self.push();
        self.ir.push(f(idx));
    }

    pub fn sum(mut self) -> Self {
        self.pop_bin_impl(None, None, MockExprIR::Sum);
        self
    }

    pub fn product(mut self) -> Self {
        self.pop_bin_impl(None, None, MockExprIR::Product);
        self
    }

    pub fn constraint(mut self, lhs: usize, rhs: usize) -> Self {
        self.pop_bin_impl(Some(lhs), Some(rhs), MockExprIR::Constraint);
        self
    }

    pub fn neg(mut self) -> Self {
        self.pop_un_impl(None, MockExprIR::Neg);
        self
    }

    pub fn sum_with(mut self, lhs: Option<usize>, rhs: Option<usize>) -> Self {
        self.pop_bin_impl(lhs, rhs, MockExprIR::Sum);
        self
    }

    pub fn product_with(mut self, lhs: Option<usize>, rhs: Option<usize>) -> Self {
        self.pop_bin_impl(lhs, rhs, MockExprIR::Product);
        self
    }
}

impl From<IRBuilder> for Vec<MockExprIR> {
    fn from(value: IRBuilder) -> Self {
        value.ir
    }
}
