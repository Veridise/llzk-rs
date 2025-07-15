use std::marker::PhantomData;

use anyhow::Result;

use crate::{
    expr::{traits::ExprLike, Expr},
    felt::IntoPrime,
    stmt::traits::StmtLike,
    vars::VarKind,
    Module, Program,
};

pub mod passes;

pub trait Optimizer<I: ?Sized, O> {
    fn optimize(&mut self, i: &I) -> Result<O>;
}

pub trait MutOptimizer<T: ?Sized> {
    fn optimize(&mut self, t: &mut T) -> Result<()>;
}

pub struct OptimizerPipelineBuilder<F: IntoPrime, K: VarKind>(OptimizerPipeline<F, K>);

impl<F: IntoPrime, K: VarKind + 'static> OptimizerPipelineBuilder<F, K> {
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

    pub fn add_module_scope_expr_pass_fn<FN, FN2>(self, f: FN) -> Self
    where
        FN: FnMut(&str) -> FN2 + 'static,
        FN2: FnMut(&dyn ExprLike) -> Result<Expr> + 'static,
    {
        self.add_pass_with_params::<AnonModuleScopedExprPass<K, FN, FN2>>(f)
    }
}

impl<F: IntoPrime, K: VarKind + 'static> Default for OptimizerPipelineBuilder<F, K> {
    fn default() -> Self {
        Self::new()
    }
}

struct AnonModuleScopedExprPass<K, FN, FN2>(FN, PhantomData<(K, FN2)>)
where
    K: VarKind,
    FN: FnMut(&str) -> FN2,
    FN2: FnMut(&dyn ExprLike) -> Result<Expr>;

impl<K, FN, FN2> From<FN> for AnonModuleScopedExprPass<K, FN, FN2>
where
    K: VarKind,
    FN: FnMut(&str) -> FN2,
    FN2: FnMut(&dyn ExprLike) -> Result<Expr>,
{
    fn from(value: FN) -> Self {
        Self(value, Default::default())
    }
}

impl<K, FN, FN2> MutOptimizer<Module<K>> for AnonModuleScopedExprPass<K, FN, FN2>
where
    K: VarKind,
    FN: FnMut(&str) -> FN2,
    FN2: FnMut(&dyn ExprLike) -> Result<Expr>,
{
    fn optimize(&mut self, module: &mut Module<K>) -> Result<()> {
        let name = module.name();
        let mut f = self.0(name);
        for stmt in module.stmts_mut() {
            apply_to_args(stmt, &mut f)?;
        }
        Ok(())
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

pub trait ModuleOptimizer<F, K: VarKind>: MutOptimizer<Module<K>> {
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

fn apply_to_args<F>(stmt: &mut dyn StmtLike, f: &mut F) -> Result<()>
where
    F: FnMut(&dyn ExprLike) -> Result<Expr>,
{
    for (idx, expr) in stmt.args().iter().enumerate() {
        let new_expr = f(expr)?;
        stmt.replace_arg(idx, new_expr)?;
    }
    Ok(())
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

impl<T, K> MutOptimizer<Module<K>> for T
where
    T: MutOptimizer<dyn StmtLike>,
    K: VarKind,
{
    fn optimize(&mut self, module: &mut Module<K>) -> Result<()> {
        for stmt in module.stmts_mut() {
            self.optimize(stmt)?;
        }
        Ok(())
    }
}
impl<T, F, K> ModuleOptimizer<F, K> for T
where
    T: MutOptimizer<Module<K>>,
    K: VarKind,
    F: IntoPrime,
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

impl<T> MutOptimizer<T> for &mut [Box<dyn MutOptimizer<T>>] {
    fn optimize(&mut self, t: &mut T) -> Result<()> {
        for pass in self.iter_mut() {
            pass.optimize(t)?;
        }
        Ok(())
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
