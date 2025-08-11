use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    rc::Rc,
};

use super::{
    events::{
        BackendEventReceiver, BackendMessages, BackendResponse, EmitStmtsMessage, EventReceiver,
    },
    func::FuncIO,
    lowering::{
        lowerable::{Lowerable, LoweringOutput},
        Lowering,
    },
    resolvers::{
        QueryResolver, ResolvedQuery, ResolvedSelector, ResolversProvider, SelectorResolver,
    },
    Backend, Codegen,
};
#[cfg(feature = "lift-field-operations")]
use crate::ir::lift::{LiftIRGuard, LiftLike};
use crate::{
    expressions::ScopedExpression,
    gates::AnyQuery,
    halo2::{Expression, Field, PrimeField, RegionIndex, Selector},
    ir::stmt::IRStmt,
    synthesis::{regions::FQN, CircuitSynthesis},
};

use anyhow::{anyhow, Result};

mod lowering;
mod vars;

pub use lowering::PicusModuleLowering;
use lowering::PicusModuleRef;
use midnight_halo2_proofs::plonk::{AdviceQuery, FixedQuery, InstanceQuery};
use num_bigint::BigUint;
use picus::{
    felt::{Felt, IntoPrime},
    opt::{
        passes::{EnsureMaxExprSizePass, FoldExprsPass},
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
    optimize: bool,
}

impl PicusParams {
    pub fn builder() -> PicusParamsBuilder {
        PicusParamsBuilder(Default::default())
    }
}

#[cfg(feature = "lift-field-operations")]
trait PicusPrimeField: LiftLike {}
#[cfg(feature = "lift-field-operations")]
impl<F: LiftLike> PicusPrimeField for F {}
#[cfg(not(feature = "lift-field-operations"))]
trait PicusPrimeField: PrimeField {}
#[cfg(not(feature = "lift-field-operations"))]
impl<F: PrimeField> PicusPrimeField for F {}

#[derive(Default)]
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

    pub fn optimize(mut self) -> Self {
        self.0.optimize = true;
        self
    }

    pub fn no_optimize(mut self) -> Self {
        self.0.optimize = false;
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
            optimize: true,
        }
    }
}

struct PicusBackendInner<L> {
    params: PicusParams,
    modules: Vec<PicusModuleRef>,
    current_scope: Option<PicusModuleLowering<L>>,
    enqueued_stmts: HashMap<RegionIndex, Vec<IRStmt<Expression<L>>>>,
    #[cfg(feature = "lift-field-operations")]
    _lift_guard: LiftIRGuard,
    _marker: PhantomData<L>,
}

#[derive(Clone)]
pub struct PicusEventReceiver<L> {
    inner: Rc<RefCell<PicusBackendInner<L>>>,
}

impl<L> PicusEventReceiver<L> {
    fn naming_convention(&self) -> NamingConvention {
        self.inner.borrow().params.naming_convention
    }
}

#[derive(Clone)]
pub struct PicusBackend<L> {
    inner: Rc<RefCell<PicusBackendInner<L>>>,
}

impl<L> PicusBackend<L> {
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

impl<L: PicusPrimeField> PicusBackend<L> {
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
        let params = &self.inner.borrow().params;
        PipelineBuilder::<L>::new()
            .add_pass::<FoldExprsPass<FeltWrap<L>>>()
            //.add_pass::<ConsolidateVarNamesPass>()
            .add_pass_with_params::<EnsureMaxExprSizePass<NamingConvention>>((
                params.expr_cutoff,
                params.naming_convention,
            ))
            .into()
    }
}

impl<L: PicusPrimeField> PicusBackendInner<L> {
    fn add_module<O>(
        &mut self,
        name: String,
        inputs: impl Iterator<Item = O>,
        outputs: impl Iterator<Item = O>,
        syn: Option<&CircuitSynthesis<L>>,
    ) -> Result<PicusModuleLowering<L>>
    where
        O: Into<VarKey> + Into<VarStr> + Clone,
    {
        let regions = syn.map(|syn| syn.regions_by_index());
        log::debug!("Region data: {regions:?}");
        let module = PicusModule::shared(name.clone(), inputs, outputs);
        if let Some(syn) = syn {
            module
                .borrow_mut()
                .add_vars(syn.seen_advice_cells().map(|((col, row), name)| {
                    VarKeySeed::new(
                        VarKeySeedInner::IO(FuncIO::Advice(*col, *row), Some(Cow::Borrowed(name))),
                        self.params.naming_convention,
                    )
                }));
        }
        self.modules.push(module.clone());
        let scope = PicusModuleLowering::new(
            module,
            self.params.lift_fixed,
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

impl<'c, L: PicusPrimeField> Codegen<'c> for PicusBackend<L> {
    type FuncOutput = PicusModuleLowering<L>;
    type F = L;
    type Output = PicusOutput<L>;

    fn define_gate_function(
        &self,
        name: &str,
        selectors: &[&Selector],
        input_queries: &[AnyQuery],
        output_queries: &[AnyQuery],
        syn: &CircuitSynthesis<L>,
    ) -> Result<Self::FuncOutput> {
        log::debug!("[Picus codegen::define_gate_function] selectors = {selectors:?}");
        log::debug!("[Picus codegen::define_gate_function] input_queries = {input_queries:?}");
        log::debug!("[Picus codegen::define_gate_function] output_queries = {output_queries:?}");
        let nc = self.naming_convention();
        self.inner.borrow_mut().add_module(
            name.to_owned(),
            mk_io(selectors.len() + input_queries.len(), VarKeySeed::arg, nc),
            mk_io(output_queries.len(), VarKeySeed::field, nc),
            Some(syn),
        )
    }

    fn define_main_function(&self, syn: &CircuitSynthesis<L>) -> Result<Self::FuncOutput> {
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
            Some(syn),
        )
    }

    fn on_scope_end(&self, scope: Self::FuncOutput) -> Result<()> {
        log::debug!("Closing scope");
        self.inner.borrow_mut().dequeue_stmts(&scope)
    }

    fn generate_output(self) -> Result<Self::Output> {
        let mut output = PicusOutput::from(self.inner.borrow().modules.clone());
        self.var_consistency_check(&output)?;
        if self.inner.borrow().params.optimize {
            self.optimization_pipeline().optimize(&mut output)?;
        }
        Ok(output)
    }

    fn define_function(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
        syn: Option<&CircuitSynthesis<Self::F>>,
    ) -> Result<Self::FuncOutput> {
        let nc = self.naming_convention();
        self.inner.borrow_mut().add_module(
            name.to_owned(),
            mk_io(inputs, VarKeySeed::arg, nc),
            mk_io(outputs, VarKeySeed::field, nc),
            syn,
        )
    }
}

impl<'c, L: PicusPrimeField> Codegen<'c> for PicusEventReceiver<L> {
    type FuncOutput = PicusModuleLowering<L>;
    type F = L;
    type Output = ();

    fn define_gate_function(
        &self,
        name: &str,
        selectors: &[&Selector],
        input_queries: &[AnyQuery],
        output_queries: &[AnyQuery],
        syn: &CircuitSynthesis<L>,
    ) -> Result<Self::FuncOutput> {
        log::debug!("[Picus codegen::define_gate_function] selectors = {selectors:?}");
        log::debug!("[Picus codegen::define_gate_function] input_queries = {input_queries:?}");
        log::debug!("[Picus codegen::define_gate_function] output_queries = {output_queries:?}");
        let nc = self.naming_convention();
        self.inner.borrow_mut().add_module(
            name.to_owned(),
            mk_io(selectors.len() + input_queries.len(), VarKeySeed::arg, nc),
            mk_io(output_queries.len(), VarKeySeed::field, nc),
            Some(syn),
        )
    }

    fn define_main_function(&self, syn: &CircuitSynthesis<L>) -> Result<Self::FuncOutput> {
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
            Some(syn),
        )
    }

    fn on_scope_end(&self, scope: Self::FuncOutput) -> Result<()> {
        log::debug!("Closing scope");
        self.inner.borrow_mut().dequeue_stmts(&scope)
    }

    fn generate_output(self) -> Result<Self::Output> {
        unreachable!()
    }

    fn define_function(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
        syn: Option<&CircuitSynthesis<Self::F>>,
    ) -> Result<Self::FuncOutput> {
        let nc = self.naming_convention();
        self.inner.borrow_mut().add_module(
            name.to_owned(),
            mk_io(inputs, VarKeySeed::arg, nc),
            mk_io(outputs, VarKeySeed::field, nc),
            syn,
        )
    }
}

#[derive(Copy, Clone)]
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

    fn resolve_advice_query(
        &self,
        query: &AdviceQuery,
    ) -> Result<(ResolvedQuery<F>, Option<Cow<'_, FQN>>)> {
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

#[derive(Copy, Clone)]
struct NullSelectorResolver;

impl SelectorResolver for NullSelectorResolver {
    fn resolve_selector(&self, _: &Selector) -> Result<ResolvedSelector> {
        Err(anyhow!(
            "Selectors are not supported in in-flight statements"
        ))
    }
}

impl<'c, L: PicusPrimeField> Backend<'c, PicusParams> for PicusBackend<L> {
    type Codegen = Self;

    fn initialize(params: PicusParams) -> Self {
        #[cfg(feature = "lift-field-operations")]
        let enable_lifting = params.lift_fixed;
        let inner: Rc<RefCell<PicusBackendInner<L>>> = Rc::new(
            PicusBackendInner {
                params,
                modules: Default::default(),
                _marker: Default::default(),
                enqueued_stmts: Default::default(),
                #[cfg(feature = "lift-field-operations")]
                _lift_guard: LiftIRGuard::lock(enable_lifting),
                current_scope: None,
            }
            .into(),
        );
        PicusBackend { inner }
    }

    fn create_codegen(&self) -> Self::Codegen {
        self.clone()
    }

    fn event_receiver(&self) -> BackendEventReceiver<<Self::Codegen as Codegen<'c>>::F> {
        BackendEventReceiver::new(PicusEventReceiver {
            inner: self.inner.clone(),
        })
    }
}

fn dequeue_stmts_impl<'s, L: PicusPrimeField>(
    scope: &'s PicusModuleLowering<L>,
    enqueued_stmts: &mut HashMap<RegionIndex, Vec<IRStmt<Expression<L>>>>,
) -> Result<()>
where
    (OnlyAdviceQueriesResolver<'s, L>, NullSelectorResolver): ResolversProvider<L> + 's,
{
    struct Dummy<F>(PhantomData<F>);

    impl<F: Field> Lowerable for Dummy<F> {
        type F = F;

        fn lower<L>(self, _: &L) -> Result<impl Into<LoweringOutput<L>>>
        where
            L: Lowering<F = Self::F> + ?Sized,
        {
            unreachable!();
            #[allow(unreachable_code)]
            Ok(())
        }
    }
    // Delete the elements waiting in the queue.
    for (region, stmts) in std::mem::take(enqueued_stmts) {
        scope.lower_stmt(IRStmt::<Dummy<L>>::comment(format!(
            "In-flight statements @ Region {} (start row: {})",
            *region,
            *scope.find_region(&region).unwrap()
        )))?;

        for stmt in stmts {
            let query_resolver = OnlyAdviceQueriesResolver::new(region, scope);
            let selector_resolver = NullSelectorResolver;
            let stmt = stmt.map(&ScopedExpression::make_ctor((
                query_resolver,
                selector_resolver,
            )));
            scope.lower_stmt(stmt)?;
        }
        scope.lower_stmt(IRStmt::<Dummy<L>>::comment(format!(
            "End of in-flight statements @ Region {} (start row: {})",
            *region,
            *scope.find_region(&region).unwrap()
        )))?;
    }
    Ok(())
}

impl<L: PicusPrimeField> PicusBackendInner<L> {
    pub fn enqueue_stmts<'s>(
        &'s mut self,
        region: RegionIndex,
        stmts: &[IRStmt<Expression<L>>],
    ) -> Result<()>
//where
    //    (OnlyAdviceQueriesResolver<'s, L>, NullSelectorResolver): ResolversProvider<L> + 's,
    {
        self.enqueued_stmts
            .entry(region)
            .or_default()
            .extend_from_slice(stmts);
        log::debug!(
            "Enqueueing {} statements. Currently enqueued: {}",
            stmts.len(),
            self.enqueued_stmts.len()
        );
        Ok(())
        //self.current_scope
        //    .as_ref()
        //    .map(|scope| dequeue_stmts_impl(scope, &mut self.enqueued_stmts))
        //    .unwrap_or_else(|| Ok(()))
    }

    pub fn dequeue_stmts<'s>(&mut self, scope: &'s PicusModuleLowering<L>) -> Result<()>
    where
        (OnlyAdviceQueriesResolver<'s, L>, NullSelectorResolver): ResolversProvider<L> + 's,
    {
        dequeue_stmts_impl(scope, &mut self.enqueued_stmts)
    }
}

//impl<L: PicusPrimeField> EventReceiver for PicusEventReceiver<L> {
//    type Message = EmitStmtsMessage<L>;
//
//    fn accept(&self, msg: &Self::Message) -> Result<()> {
//        self.inner.borrow_mut().enqueue_stmts(msg.0, &msg.1)
//    }
//}

impl<F: PicusPrimeField> EventReceiver for PicusEventReceiver<F> {
    type Message = BackendMessages<F>;

    fn accept(
        &self,
        msg: &Self::Message,
    ) -> Result<<Self::Message as super::events::Message>::Response> {
        match msg {
            BackendMessages::EmitStmts(msg) => self
                .inner
                .borrow_mut()
                .enqueue_stmts(msg.0, &msg.1)
                .map(BackendResponse::EmitStmts),
        }
    }
}
