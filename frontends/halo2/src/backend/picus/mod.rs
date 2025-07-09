use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    rc::Rc,
};

use super::{
    events::{EmitStmtsMessage, EventReceiver},
    func::FuncIO,
    lowering::Lowering as _,
    resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    Backend, Codegen,
};
use crate::{
    gates::AnyQuery,
    halo2::{Advice, Field, Instance, PrimeField, Selector},
    ir::CircuitStmt,
    synthesis::regions::FQN,
    CircuitIO, EventSender, LiftLike,
};
use anyhow::{anyhow, Result};

mod lowering;
mod vars;

pub use lowering::PicusModuleLowering;
use lowering::{PicusModuleRef, VarEqvClassesRef};
use midnight_halo2_proofs::plonk::{AdviceQuery, FixedQuery, InstanceQuery};
use num_bigint::BigUint;
use picus::{
    expr::Expr,
    felt::{Felt, IntoPrime},
    opt::{EnsureMaxExprSizePass, FoldExprsPass, MutOptimizer as _},
    vars::VarStr,
    ModuleLike as _,
};
use vars::{VarKey, VarKeySeed};

pub type PicusModule = picus::Module<VarKey>;
pub type PicusOutput<F> = picus::Program<FeltWrap<F>, VarKey>;
type PipelineBuilder<F> = picus::opt::OptimizerPipelineBuilder<FeltWrap<F>, VarKey>;
type Pipeline<F> = picus::opt::OptimizerPipeline<FeltWrap<F>, VarKey>;

pub struct PicusParams {
    expr_cutoff: usize,
    entrypoint: String,
    lift_fixed: bool,
}

impl PicusParams {
    pub fn builder() -> PicusParamsBuilder {
        PicusParamsBuilder(Default::default())
    }
}

pub struct FeltWrap<F: PrimeField>(F);

impl<F: PrimeField> From<F> for FeltWrap<F> {
    fn from(value: F) -> Self {
        Self(value)
    }
}

impl<F: PrimeField> From<&F> for FeltWrap<F> {
    fn from(value: &F) -> Self {
        Self(*value)
    }
}

impl<F: PrimeField> From<FeltWrap<F>> for Felt {
    fn from(wrap: FeltWrap<F>) -> Felt {
        let r = wrap.0.to_repr();
        Felt::new(BigUint::from_bytes_le(r.as_ref()))
    }
}

impl<F: PrimeField> IntoPrime for FeltWrap<F> {
    fn prime() -> Felt {
        let mut f = FeltWrap(-F::ONE).into();
        f += 1;
        f
    }
}

#[derive(Default)]
pub struct PicusParamsBuilder(PicusParams);

impl PicusParamsBuilder {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn expr_cutoff(self, expr_cutoff: usize) -> Self {
        let mut p = self.0;
        p.expr_cutoff = expr_cutoff;
        Self(p)
    }

    pub fn entrypoint(self, name: &str) -> Self {
        let mut p = self.0;
        p.entrypoint = name.to_owned();
        Self(p)
    }

    pub fn no_lift_fixed(self) -> Self {
        let mut p = self.0;
        p.lift_fixed = false;
        Self(p)
    }

    pub fn lift_fixed(self) -> Self {
        let mut p = self.0;
        p.lift_fixed = true;
        Self(p)
    }
}

impl From<PicusParamsBuilder> for PicusParams {
    fn from(builder: PicusParamsBuilder) -> PicusParams {
        builder.0
    }
}

impl Default for PicusParams {
    fn default() -> Self {
        Self {
            expr_cutoff: 10,
            entrypoint: "Main".to_owned(),
            lift_fixed: false,
        }
    }
}

struct PicusBackendInner<L> {
    params: PicusParams,
    modules: Vec<PicusModuleRef>,
    eqv_vars: HashMap<String, VarEqvClassesRef>,
    current_scope: Option<PicusModuleLowering<L>>,
    _marker: PhantomData<L>,
}

#[derive(Clone)]
pub struct PicusEventReceiver<L> {
    inner: Rc<RefCell<PicusBackendInner<L>>>,
}

pub struct PicusBackend<L> {
    inner: Rc<RefCell<PicusBackendInner<L>>>,
}

fn mk_io<F, I, O>(count: usize, f: F) -> impl Iterator<Item = O>
where
    O: Into<VarKey> + Into<VarStr>,
    I: From<usize>,
    F: Fn(I) -> O + 'static,
{
    (0..count).map(move |i| f(i.into()))
}

impl<L: PrimeField> PicusBackend<L> {
    pub fn event_receiver(&self) -> PicusEventReceiver<L> {
        PicusEventReceiver {
            inner: self.inner.clone(),
        }
    }

    fn var_consistency_check(&self, output: &PicusOutput<L>) -> Result<()> {
        // Var consistency check
        for module in output.modules() {
            let vars = module.vars();
            // Get the set of io variables, without the fqn.
            // This set will have all the circuit cells that have been queried and resolved
            // during lowering.
            let io_vars = vars
                .keys()
                .filter_map(|k| match k {
                    VarKey::IO(func_io) => Some(*func_io),
                    _ => None,
                })
                .collect::<HashSet<_>>();

            // The set of io variables, with names, should be the same length.
            let io_var_count = vars
                .iter()
                .filter_map(|(k, v)| match k {
                    VarKey::IO(_) => Some(v),
                    _ => None,
                })
                .count();
            if io_vars.len() != io_var_count {
                // Inconsistency. Let's see which ones.
                let mut dups = HashMap::<FuncIO, Vec<&VarStr>>::new();
                for (k, v) in vars {
                    if let VarKey::IO(f) = k {
                        dups.entry(*f).or_default().push(v);
                    }
                }

                let dups = dups;
                for (k, names) in dups {
                    if names.len() == 1 {
                        continue;
                    }
                    log::error!("Mismatched variable! (key = {k:?}) (names = {names:?})");
                }
                anyhow::bail!(
                    "Inconsistency detected in circuit variables. Was expecting {} IO variables by {} were generated",
                    io_vars.len(),
                    io_var_count
                );
            }
        }
        Ok(())
    }

    fn optimization_pipeline(&self) -> Pipeline<L> {
        PipelineBuilder::<L>::new()
            .add_pass::<FoldExprsPass>()
            .add_pass_with_params::<EnsureMaxExprSizePass>(self.inner.borrow().params.expr_cutoff)
            //.add_module_scope_expr_pass_fn(|name| {
            //    let eqv_classes = self
            //        .eqv_vars
            //        .borrow()
            //        .get(name)
            //        .cloned()
            //        .unwrap_or_default();
            //    let renames = eqv_classes.rename_sets();
            //    move |expr| Ok(expr.renamed(&renames).unwrap_or_else(|| expr.wrap()))
            //})
            .into()
    }
}

impl<L: LiftLike> PicusBackendInner<L> {
    fn add_module<O>(
        &mut self,
        name: String,
        inputs: impl Iterator<Item = O>,
        outputs: impl Iterator<Item = O>,
    ) -> Result<PicusModuleLowering<L>>
    where
        O: Into<VarKey> + Into<VarStr> + Clone,
    {
        let module = PicusModule::shared(name.clone(), inputs, outputs);
        self.modules.push(module.clone());
        let eqv_vars = VarEqvClassesRef::default();
        self.eqv_vars.insert(name, eqv_vars.clone());
        let scope = PicusModuleLowering::new(module, self.params.lift_fixed, eqv_vars);
        self.current_scope = Some(scope.clone());
        Ok(scope)
    }

    fn entrypoint(&self) -> String {
        self.params.entrypoint.clone()
    }
}

macro_rules! codegen_impl {
    ($t:ident) => {
        impl<'c, L: LiftLike> Codegen<'c> for $t<L> {
            type FuncOutput = PicusModuleLowering<L>;
            type F = L;

            fn define_gate_function<'f>(
                &self,
                name: &str,
                selectors: &[&Selector],
                queries: &[AnyQuery],
            ) -> Result<Self::FuncOutput>
            where
                Self::FuncOutput: 'f,
                'c: 'f,
            {
                self.inner.borrow_mut().add_module(
                    name.to_owned(),
                    mk_io(selectors.len() + queries.len(), VarKeySeed::arg),
                    mk_io(0, VarKeySeed::field),
                )
            }

            fn define_main_function<'f>(
                &self,
                advice_io: &CircuitIO<Advice>,
                instance_io: &CircuitIO<Instance>,
            ) -> Result<Self::FuncOutput>
            where
                Self::FuncOutput: 'f,
                'c: 'f,
            {
                let ep = self.inner.borrow().entrypoint();
                self.inner.borrow_mut().add_module(
                    ep,
                    mk_io(
                        instance_io.inputs().len() + advice_io.inputs().len(),
                        VarKeySeed::arg,
                    ),
                    mk_io(
                        instance_io.outputs().len() + advice_io.outputs().len(),
                        VarKeySeed::field,
                    ),
                )
            }

            fn on_current_scope<FN, FO>(&self, f: FN) -> Option<FO>
            where
                FN: FnOnce(
                    &Self::FuncOutput,
                    &dyn QueryResolver<Self::F>,
                    &dyn SelectorResolver,
                ) -> FO,
            {
                self.inner.borrow().current_scope.as_ref().map(|scope| {
                    f(
                        scope,
                        &OnlyAdviceQueriesResolver::default(),
                        &NullSelectorResolver,
                    )
                })
            }
        }
    };
}

codegen_impl!(PicusBackend);
codegen_impl!(PicusEventReceiver);

#[derive(Default)]
struct OnlyAdviceQueriesResolver<F>(PhantomData<F>);

impl<F: Field> QueryResolver<F> for OnlyAdviceQueriesResolver<F> {
    fn resolve_fixed_query(&self, _: &FixedQuery) -> Result<ResolvedQuery<F>> {
        Err(anyhow!(
            "Fixed cells are not supported in in-flight statements"
        ))
    }

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<(ResolvedQuery<F>, Option<FQN>)> {
        Ok((
            ResolvedQuery::IO(FuncIO::Temp(
                query.column_index(),
                query.rotation().0.try_into()?,
            )),
            None,
        ))
    }

    fn resolve_instance_query(&self, _: &InstanceQuery) -> Result<ResolvedQuery<F>> {
        Err(anyhow!(
            "Instance cells are not supported in in-flight statements"
        ))
    }
}

struct NullSelectorResolver;

impl SelectorResolver for NullSelectorResolver {
    fn resolve_selector(&self, _: &Selector) -> Result<ResolvedSelector> {
        Err(anyhow!(
            "Selectors are not supported in statements emitted on flight"
        ))
    }
}

impl<'c, L: LiftLike> Backend<'c, PicusParams, PicusOutput<L>> for PicusBackend<L> {
    fn initialize(params: PicusParams) -> Self {
        let inner: Rc<RefCell<PicusBackendInner<L>>> = Rc::new(
            PicusBackendInner {
                params,
                eqv_vars: Default::default(),
                modules: Default::default(),
                _marker: Default::default(),
                current_scope: None,
            }
            .into(),
        );
        PicusBackend { inner }
    }

    fn generate_output(self) -> Result<PicusOutput<Self::F>> {
        let mut output = PicusOutput::from(self.inner.borrow().modules.clone());
        self.var_consistency_check(&output)?;
        self.optimization_pipeline().optimize(&mut output)?;
        Ok(output)
    }
}

impl<L: LiftLike> EventReceiver for PicusEventReceiver<L> {
    type Message = EmitStmtsMessage<L>;

    fn accept(&self, msg: &Self::Message) -> Result<()> {
        self.on_current_scope(move |scope, qr, sr| -> Result<()> {
            let stmts = msg.0.iter().map(|stmt| {
                Ok(match stmt {
                    CircuitStmt::ConstraintCall(callee, inputs, outputs) => {
                        CircuitStmt::ConstraintCall(
                            callee.clone(),
                            scope.lower_exprs(inputs, qr, sr)?,
                            scope.lower_exprs(outputs, qr, sr)?,
                        )
                    }
                    CircuitStmt::Constraint(op, lhs, rhs) => CircuitStmt::Constraint(
                        *op,
                        scope.lower_expr(lhs, qr, sr)?,
                        scope.lower_expr(rhs, qr, sr)?,
                    ),
                    CircuitStmt::Comment(s) => CircuitStmt::Comment(s.clone()),
                })
            });
            //for stmt in &msg.0 {
            //    let lowered_stmt = match stmt {};
            self.lower_stmts(scope, stmts)
            //}
        })
        .ok_or_else(|| anyhow!("No scope where to emit statements"))?
    }
}
