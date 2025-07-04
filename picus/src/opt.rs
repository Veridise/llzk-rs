use std::marker::PhantomData;

use anyhow::{anyhow, Result};

use crate::{
    expr::{
        self, known_var,
        traits::{ConstraintEmitter, ExprLike},
        Expr,
    },
    felt::IntoPrime,
    stmt::{self, traits::StmtLike, Stmt},
    vars::{Temp, VarKind, VarStr},
    Module, Program,
};

pub trait Optimizer<I: ?Sized, O> {
    fn optimize(&mut self, i: &I) -> Result<O>;
}

pub trait MutOptimizer<T: ?Sized> {
    fn optimize(&mut self, t: &mut T) -> Result<()>;
}

pub struct OptimizerPipelineBuilder<F: IntoPrime, K: VarKind>(OptimizerPipeline<F, K>);

impl<F: IntoPrime, K: VarKind> OptimizerPipelineBuilder<F, K> {
    pub fn new() -> Self {
        Self(OptimizerPipeline {
            passes: Default::default(),
        })
    }

    pub fn add_pass_with_params<P: ProgramOptimizer<F, K> + 'static>(
        self,
        params: impl Into<P>,
    ) -> Self {
        let mut b = self;
        b.0.passes.push(P::create(params));
        b
    }

    pub fn add_pass<P: ProgramOptimizer<F, K> + Default + 'static>(self) -> Self {
        let mut b = self;
        b.0.passes.push(P::create(P::default()));
        b
    }
}

pub struct OptimizerPipeline<F: IntoPrime, K: VarKind> {
    passes: Vec<Box<dyn MutOptimizer<Program<F, K>>>>,
}

impl<F: IntoPrime, K: VarKind> From<OptimizerPipelineBuilder<F, K>> for OptimizerPipeline<F, K> {
    fn from(value: OptimizerPipelineBuilder<F, K>) -> Self {
        value.0
    }
}

impl<F: IntoPrime, K: VarKind> MutOptimizer<Program<F, K>> for OptimizerPipeline<F, K> {
    fn optimize(&mut self, program: &mut Program<F, K>) -> Result<()> {
        self.passes.as_mut_slice().optimize(program)
    }
}

pub trait ExprOptimizer: Optimizer<dyn ExprLike, Expr> {
    fn create<I>(i: I) -> Box<dyn Optimizer<dyn ExprLike, Expr>>
    where
        I: Into<Self>,
        Self: Sized + 'static,
    {
        Box::new(i.into())
    }
}

pub trait StmtOptimizer: MutOptimizer<dyn StmtLike> {
    fn create<I>(i: I) -> Box<dyn MutOptimizer<dyn StmtLike>>
    where
        I: Into<Self>,
        Self: Sized + 'static,
    {
        Box::new(i.into())
    }
}

pub trait ModuleOptimizer<K: VarKind>: MutOptimizer<Module<K>> {
    fn create<I>(i: I) -> Box<dyn MutOptimizer<Module<K>>>
    where
        I: Into<Self>,
        Self: Sized + 'static,
    {
        Box::new(i.into())
    }
}

pub trait ProgramOptimizer<F: IntoPrime, K: VarKind>: MutOptimizer<Program<F, K>> {
    fn create<I>(i: I) -> Box<dyn MutOptimizer<Program<F, K>>>
    where
        I: Into<Self>,
        Self: Sized + 'static,
    {
        Box::new(i.into())
    }
}

impl<T> ExprOptimizer for T where T: Optimizer<dyn ExprLike, Expr> {}

impl<T> MutOptimizer<dyn StmtLike> for T
where
    T: Optimizer<dyn ExprLike + 'static, Expr>,
{
    fn optimize(&mut self, stmt: &mut (dyn StmtLike + 'static)) -> Result<()> {
        for (idx, expr) in stmt.args().iter().enumerate() {
            let new_expr = self.optimize(expr)?;
            stmt.replace_arg(idx, new_expr)?;
        }

        Ok(())
    }
}
impl<T> StmtOptimizer for T where T: MutOptimizer<dyn StmtLike> {}

impl<T, K: VarKind> MutOptimizer<Module<K>> for T
where
    T: MutOptimizer<dyn StmtLike>,
{
    fn optimize(&mut self, module: &mut Module<K>) -> Result<()> {
        for stmt in module.stmts_mut() {
            self.optimize(stmt)?;
        }
        Ok(())
    }
}
impl<T, K> ModuleOptimizer<K> for T
where
    T: MutOptimizer<Module<K>>,
    K: VarKind,
{
}

impl<T, F: IntoPrime, K: VarKind> MutOptimizer<Program<F, K>> for T
where
    T: MutOptimizer<Module<K>>,
{
    fn optimize(&mut self, program: &mut Program<F, K>) -> Result<()> {
        for module in program.modules_mut() {
            self.optimize(module)?;
        }
        Ok(())
    }
}

impl<T, F, K> ProgramOptimizer<F, K> for T
where
    T: MutOptimizer<Program<F, K>>,
    K: VarKind,
    F: IntoPrime,
{
}

//impl<I, O> Optimizer<I, O> for &mut [Box<dyn Optimizer<I, O>>]
//where
//    I: Into<O>,
//    O: AsRef<I>,
//{
//    fn optimize(&mut self, i: &I) -> Result<O> {
//        let mut iter = self.iter_mut();
//        let head = iter
//            .next()
//            .ok_or_else(|| anyhow!("No optimizations available"))
//            .and_then(|opt| opt.optimize(i))?;
//        iter.try_fold(head, |acc, opt| opt.optimize(acc.as_ref()))
//    }
//}

impl<T> MutOptimizer<T> for &mut [Box<dyn MutOptimizer<T>>] {
    fn optimize(&mut self, t: &mut T) -> Result<()> {
        for pass in self.iter_mut() {
            pass.optimize(t)?;
        }
        Ok(())
    }
}

pub struct EnsureMaxExprSizePass {
    limit: usize,
}

impl From<usize> for EnsureMaxExprSizePass {
    fn from(limit: usize) -> Self {
        Self { limit }
    }
}

impl ConstraintEmitter for Vec<Stmt> {
    fn emit(&mut self, lhs: Expr, rhs: Expr) {
        self.push(stmt::constrain(expr::eq(&lhs, &rhs)))
    }
}

impl<K: Temp> MutOptimizer<Module<K>> for EnsureMaxExprSizePass {
    fn optimize(&mut self, t: &mut Module<K>) -> Result<()> {
        let temporaries = [K::temp()]
            .into_iter()
            .cycle()
            .map(|t| -> VarStr { t.into() });
        let mut new_constraints = vec![];
        let mut r#impl = EnsureMaxExprSizePassImpl {
            limit: self.limit,
            emitter: &mut new_constraints,
            temporaries,
        };

        MutOptimizer::optimize(&mut r#impl, t)?;

        t.add_stmts(&new_constraints);
        Ok(())
    }
}

struct EnsureMaxExprSizePassImpl<'a, E, T> {
    limit: usize,
    emitter: &'a mut E,
    temporaries: T,
}

impl<E, T> Optimizer<dyn ExprLike, Expr> for EnsureMaxExprSizePassImpl<'_, E, T>
where
    E: ConstraintEmitter,
    T: Iterator<Item = VarStr>,
{
    /// If the expression's size is larger than the threshold
    /// replaces the expression with a temporary and emit a constraint that
    /// equates that fresh temporary with the expression.
    /// If not returns itself.
    fn optimize(&mut self, expr: &dyn ExprLike) -> Result<Expr> {
        if expr.size() < self.limit {
            return Ok(expr.wrap());
        }
        let temp = known_var(
            &self
                .temporaries
                .next()
                .ok_or_else(|| anyhow!("Temporaries generator is exhausted"))?,
        );
        self.emitter.emit(temp.clone(), expr.wrap());
        Ok(temp)
    }
}

//pub struct EnsureMaxExprSizePass<'a, C, I> {
//    params: EnsureMaxExprSize<'a, C, I>,
//}
//
//impl<'a, C, I> From<EnsureMaxExprSize<'a, C, I>> for EnsureMaxExprSizePass<'a, C, I> {
//    fn from(params: EnsureMaxExprSize<'a, C, I>) -> Self {
//        Self { params }
//    }
//}
//
//impl<C, I> Optimizer<dyn ExprLike, Expr> for EnsureMaxExprSizePass<'_, C, I>
//where
//    C: ConstraintEmitter,
//    I: Iterator<Item = VarStr>,
//{
//}

#[derive(Default)]
pub struct FoldExprsPass;

impl Optimizer<dyn ExprLike, Expr> for FoldExprsPass {
    fn optimize(&mut self, i: &dyn ExprLike) -> Result<Expr> {
        Ok(i.fold().unwrap_or_else(|| i.wrap()))
    }
}
