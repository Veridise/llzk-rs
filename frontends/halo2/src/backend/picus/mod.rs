use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    rc::Rc,
};

use super::{
    events::{EmitStmtsMessage, EventReceiver},
    func::FuncIO,
    lower_stmts,
    lowering::Lowering as _,
    resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    Backend, Codegen,
};
use crate::{
    gates::AnyQuery,
    halo2::{Advice, Expression, Field, Instance, PrimeField, RegionIndex, Selector, Value},
    ir::{lift::LiftIRGuard, CircuitStmt},
    synthesis::{regions::FQN, CircuitSynthesis},
    CircuitIO, EventSender, LiftLike,
};
use anyhow::{anyhow, Result};

mod lowering;
mod vars;

pub use lowering::PicusModuleLowering;
use lowering::{PicusExpr, PicusModuleRef, VarEqvClassesRef};
use midnight_halo2_proofs::plonk::{AdviceQuery, FixedQuery, InstanceQuery};
use num_bigint::BigUint;
use picus::{
    felt::{Felt, IntoPrime},
    opt::{
        passes::{ConsolidateVarNamesPass, EnsureMaxExprSizePass, FoldExprsPass},
        MutOptimizer as _,
    },
    vars::VarStr,
    ModuleWithVars as _,
};
use vars::{NamingConvention, VarKey, VarKeySeed, VarKeySeedInner};

pub type PicusModule = picus::Module<VarKey>;
pub type PicusOutput<F> = picus::Program<FeltWrap<F>, VarKey>;
type PipelineBuilder<F> = picus::opt::OptimizerPipelineBuilder<FeltWrap<F>, VarKey>;
type Pipeline<F> = picus::opt::OptimizerPipeline<FeltWrap<F>, VarKey>;

pub struct PicusParams {
    expr_cutoff: usize,
    entrypoint: String,
    lift_fixed: bool,
    naming_convention: NamingConvention,
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

    pub fn short_names(mut self) -> Self {
        self.0.naming_convention = NamingConvention::Short;
        self
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
            naming_convention: NamingConvention::Default,
        }
    }
}

struct PicusBackendInner<'a, L> {
    params: PicusParams,
    modules: Vec<PicusModuleRef>,
    eqv_vars: HashMap<String, VarEqvClassesRef>,
    current_scope: Option<PicusModuleLowering<L>>,
    enqueued_stmts: HashMap<RegionIndex, Vec<CircuitStmt<Expression<L>>>>,
    _lift_guard: LiftIRGuard<'a>,
    _marker: PhantomData<L>,
}

#[derive(Clone)]
pub struct PicusEventReceiver<'a, L> {
    inner: Rc<RefCell<PicusBackendInner<'a, L>>>,
}

impl<L> PicusEventReceiver<'_, L> {
    fn naming_convention(&self) -> NamingConvention {
        self.inner.borrow().params.naming_convention
    }
}

pub struct PicusBackend<'a, L> {
    inner: Rc<RefCell<PicusBackendInner<'a, L>>>,
}

impl<L> PicusBackend<'_, L> {
    fn naming_convention(&self) -> NamingConvention {
        self.inner.borrow().params.naming_convention
    }
}

fn mk_io<F, I, O, C>(count: usize, f: F, c: C) -> impl Iterator<Item = O>
where
    O: Into<VarKey> + Into<VarStr>,
    I: From<usize>,
    F: Fn(I, C) -> O + 'static,
    C: Copy,
{
    (0..count).map(move |i| f(i.into(), c))
}

impl<'a, L: PrimeField> PicusBackend<'a, L> {
    pub fn event_receiver(&self) -> PicusEventReceiver<'a, L> {
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
        let params = &self.inner.borrow().params;
        PipelineBuilder::<L>::new()
            .add_pass::<FoldExprsPass>()
            //.add_pass::<ConsolidateVarNamesPass>()
            .add_pass_with_params::<EnsureMaxExprSizePass<NamingConvention>>((
                params.expr_cutoff,
                params.naming_convention,
            ))
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

impl<L: LiftLike> PicusBackendInner<'_, L> {
    fn add_module<O>(
        &mut self,
        name: String,
        inputs: impl Iterator<Item = O>,
        outputs: impl Iterator<Item = O>,
        syn: &CircuitSynthesis<L>,
    ) -> Result<PicusModuleLowering<L>>
    where
        O: Into<VarKey> + Into<VarStr> + Clone,
    {
        let regions = syn.regions_by_index();
        log::debug!("Region data: {regions:?}");
        let module = PicusModule::shared(name.clone(), inputs, outputs);
        module
            .borrow_mut()
            .add_vars(syn.seen_advice_cells().map(|((col, row), name)| {
                VarKeySeed::new(
                    VarKeySeedInner::IO(FuncIO::Advice(*col, *row), Some(name.clone())),
                    self.params.naming_convention,
                )
            }));
        self.modules.push(module.clone());
        let eqv_vars = VarEqvClassesRef::default();
        self.eqv_vars.insert(name.clone(), eqv_vars.clone());
        let scope = PicusModuleLowering::new(
            module,
            self.params.lift_fixed,
            eqv_vars,
            regions,
            self.params.naming_convention,
        );
        log::debug!("Setting the scope to {name}");
        self.current_scope = Some(scope.clone());
        Ok(scope)
    }

    fn entrypoint(&self) -> String {
        self.params.entrypoint.clone()
    }
}

macro_rules! codegen_impl {
    ($t:ident) => {
        impl<'c, L: LiftLike> Codegen<'c> for $t<'c, L> {
            type FuncOutput = PicusModuleLowering<L>;
            type F = L;

            fn define_gate_function<'f>(
                &self,
                name: &str,
                selectors: &[&Selector],
                queries: &[AnyQuery],
                syn: &CircuitSynthesis<L>,
            ) -> Result<Self::FuncOutput>
            where
                Self::FuncOutput: 'f,
                'c: 'f,
            {
                let nc = self.naming_convention();
                self.inner.borrow_mut().add_module(
                    name.to_owned(),
                    mk_io(selectors.len() + queries.len(), VarKeySeed::arg, nc),
                    mk_io(0, VarKeySeed::field, nc),
                    syn,
                )
            }

            fn define_main_function<'f>(
                &self,
                syn: &CircuitSynthesis<L>,
            ) -> Result<Self::FuncOutput>
            where
                Self::FuncOutput: 'f,
                'c: 'f,
            {
                let ep = self.inner.borrow().entrypoint();
                let instance_io = syn.instance_io();
                let advice_io = syn.advice_io();
                let nc = self.naming_convention();
                self.inner.borrow_mut().add_module(
                    ep,
                    mk_io(
                        instance_io.inputs().len() + advice_io.inputs().len(),
                        VarKeySeed::arg,
                        nc,
                    ),
                    mk_io(
                        instance_io.outputs().len() + advice_io.outputs().len(),
                        VarKeySeed::field,
                        nc,
                    ),
                    syn,
                )
            }

            fn on_scope_end(&self, scope: &Self::FuncOutput) -> Result<()> {
                log::debug!("Closing scope");
                self.inner.borrow_mut().dequeue_stmts(scope)
            }
        }
    };
}

codegen_impl!(PicusBackend);
codegen_impl!(PicusEventReceiver);

struct OnlyAdviceQueriesResolver<'s, F> {
    region: RegionIndex,
    scope: &'s PicusModuleLowering<F>,
}

impl<'s, F> OnlyAdviceQueriesResolver<'s, F> {
    pub fn new(region: RegionIndex, scope: &'s PicusModuleLowering<F>) -> Self {
        Self { region, scope }
    }
}

impl<F: Field> QueryResolver<F> for OnlyAdviceQueriesResolver<'_, F> {
    fn resolve_fixed_query(&self, _: &FixedQuery) -> Result<ResolvedQuery<F>> {
        Err(anyhow!(
            "Fixed cells are not supported in in-flight statements"
        ))
    }

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<(ResolvedQuery<F>, Option<FQN>)> {
        let offset: usize = query.rotation().0.try_into()?;
        let start = self
            .scope
            .find_region(&self.region)
            .ok_or_else(|| anyhow!("Unrecognized region {:?}", self.region))?;
        Ok((
            ResolvedQuery::IO(FuncIO::Advice(query.column_index(), *start + offset)),
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
            "Selectors are not supported in in-flight statements"
        ))
    }
}

impl<'c, L: LiftLike> Backend<'c, PicusParams, PicusOutput<L>> for PicusBackend<'c, L> {
    fn initialize(params: PicusParams) -> Self {
        let enable_lifting = params.lift_fixed;
        let inner: Rc<RefCell<PicusBackendInner<L>>> = Rc::new(
            PicusBackendInner {
                params,
                eqv_vars: Default::default(),
                modules: Default::default(),
                _marker: Default::default(),
                enqueued_stmts: Default::default(),
                _lift_guard: LiftIRGuard::lock(enable_lifting),
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

fn lower_stmt<L: LiftLike>(
    stmt: &CircuitStmt<Expression<L>>,
    scope: &PicusModuleLowering<L>,
    qr: &dyn QueryResolver<L>,
    sr: &dyn SelectorResolver,
) -> Result<CircuitStmt<Value<PicusExpr>>> {
    Ok(match stmt {
        CircuitStmt::ConstraintCall(callee, inputs, outputs) => CircuitStmt::ConstraintCall(
            callee.clone(),
            scope.lower_exprs(inputs, qr, sr)?,
            scope.lower_exprs(outputs, qr, sr)?,
        ),
        CircuitStmt::Constraint(op, lhs, rhs) => CircuitStmt::Constraint(
            *op,
            scope.lower_expr(lhs, qr, sr)?,
            scope.lower_expr(rhs, qr, sr)?,
        ),
        CircuitStmt::Comment(s) => CircuitStmt::Comment(s.clone()),
    })
}

fn dequeue_stmts_impl<L: LiftLike>(
    scope: &PicusModuleLowering<L>,
    enqueued_stmts: &mut HashMap<RegionIndex, Vec<CircuitStmt<Expression<L>>>>,
) -> Result<()> {
    lower_stmts(
        scope,
        // Delete the elements waiting in the queue.
        std::mem::take(enqueued_stmts)
            .into_iter()
            .flat_map(|(region, stmts)| {
                [Ok(CircuitStmt::Comment(format!(
                    "In-flight statements @ Region {} (start row: {})",
                    *region,
                    *scope.find_region(&region).unwrap()
                )))]
                .into_iter()
                .chain(stmts.into_iter().map(move |stmt| {
                    let query_resolver = OnlyAdviceQueriesResolver::new(region, scope);
                    let selector_resolver = NullSelectorResolver;
                    lower_stmt(&stmt, scope, &query_resolver, &selector_resolver)
                }))
                .chain([Ok(CircuitStmt::Comment(format!(
                    "End of in-flight statements @ Region {} (start row: {})",
                    *region,
                    *scope.find_region(&region).unwrap()
                )))])
            }),
    )
}

impl<L: LiftLike> PicusBackendInner<'_, L> {
    pub fn enqueue_stmts(
        &mut self,
        region: RegionIndex,
        stmts: &[CircuitStmt<Expression<L>>],
    ) -> Result<()> {
        self.enqueued_stmts
            .entry(region)
            .or_default()
            .extend_from_slice(&stmts);
        log::debug!(
            "Enqueueing {} statements. Currently enqueued: {}",
            stmts.len(),
            self.enqueued_stmts.len()
        );
        self.current_scope
            .as_ref()
            .map(|scope| dequeue_stmts_impl(scope, &mut self.enqueued_stmts))
            .unwrap_or_else(|| Ok(()))
    }

    pub fn dequeue_stmts(&mut self, scope: &PicusModuleLowering<L>) -> Result<()> {
        dequeue_stmts_impl(scope, &mut self.enqueued_stmts)
    }
}

impl<L: LiftLike> EventReceiver for PicusEventReceiver<'_, L> {
    type Message = EmitStmtsMessage<L>;

    fn accept(&self, msg: &Self::Message) -> Result<()> {
        self.inner.borrow_mut().enqueue_stmts(msg.0, &msg.1)
    }
}
