use crate::{
    gates::{compute_gate_arity, AnyQuery},
    halo2::{
        Advice, AdviceQuery, Any, Column, Field, FixedQuery, Gate, Instance, InstanceQuery,
        Rotation, Selector, Value,
    },
    ir::{BinaryBoolOp, CircuitStmt},
    synthesis::{
        regions::{RegionRow, Row, FQN},
        CircuitSynthesis,
    },
    CircuitIO, CircuitWithIO,
};
use anyhow::{anyhow, Result};

pub mod events;
pub mod func;
pub mod llzk;
pub mod lowering;
pub mod picus;
pub mod resolvers;

use events::{EmitStmtsMessage, EventReceiver};
use func::ArgNo;
use lowering::Lowering;
use midnight_halo2_proofs::plonk::Expression;
use resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver};

struct GateScopedResolver<'a> {
    selectors: &'a [&'a Selector],
    queries: &'a [AnyQuery],
}

fn resolve<'a, A, B, I, O>(it: I, b: &B, base: usize, err: &'static str) -> Result<O>
where
    A: PartialEq<B> + 'a,
    I: Iterator<Item = &'a A>,
    O: From<ArgNo>,
{
    it.enumerate()
        .find_map(|(idx, a)| {
            if a == b {
                Some(ArgNo::from(idx + base))
            } else {
                None
            }
        })
        .map(From::from)
        .ok_or(anyhow!(err))
}

impl<F: Field> QueryResolver<F> for GateScopedResolver<'_> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        resolve(
            self.queries.iter(),
            query,
            self.selectors.len(),
            "Query as argument not found",
        )
    }

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<(ResolvedQuery<F>, Option<FQN>)> {
        Ok((
            resolve(
                self.queries.iter(),
                query,
                self.selectors.len(),
                "Query as argument not found",
            )?,
            None,
        ))
    }

    fn resolve_instance_query(&self, query: &InstanceQuery) -> Result<ResolvedQuery<F>> {
        resolve(
            self.queries.iter(),
            query,
            self.selectors.len(),
            "Query as argument not found",
        )
    }
}

impl SelectorResolver for GateScopedResolver<'_> {
    fn resolve_selector(&self, selector: &Selector) -> Result<ResolvedSelector> {
        resolve(
            self.selectors.iter().copied(),
            selector,
            0,
            "Selector as argument not found",
        )
    }
}

pub trait CodegenStrategy: Default {
    fn codegen<'c, 'a, F, P, O, B>(&self, backend: &B, syn: &CircuitSynthesis<F>) -> Result<()>
    where
        F: Field,
        P: Default,
        B: Backend<'c, P, O, F = F>;

    fn inter_region_constraints<'c, F, L>(
        &self,
        scope: &'c L,
        syn: &'c CircuitSynthesis<F>,
    ) -> impl Iterator<Item = Result<CircuitStmt<Value<L::CellOutput>>>> + 'c
    where
        F: Field,
        L: Lowering<F = F>,
    {
        let lower_cell = |(col, row): &(Column<Any>, usize)| -> Result<Value<L::CellOutput>> {
            let q = col.query_cell::<L::F>(Rotation::cur());
            let row = Row::new(*row, syn.advice_io(), syn.instance_io());
            scope.lower_expr(&q, &row, &row)
        };
        let mut constraints = syn.constraints().collect::<Vec<_>>();
        constraints.sort();
        constraints.into_iter().map(move |(from, to)| {
            Ok(CircuitStmt::Constraint(
                BinaryBoolOp::Eq,
                lower_cell(from)?,
                lower_cell(to)?,
            ))
        })
    }
}

#[derive(Default)]
pub struct CallGatesStrat;

impl CallGatesStrat {
    fn create_call_stmt<F, L>(
        &self,
        scope: &L,
        name: &str,
        selectors: Vec<&Selector>,
        queries: Vec<AnyQuery>,
        r: &RegionRow<F>,
    ) -> Result<CircuitStmt<Value<L::CellOutput>>>
    where
        F: Field,
        L: Lowering<F = F>,
    {
        Ok(CircuitStmt::ConstraintCall(
            name.to_owned(),
            scope.lower_selectors(&selectors, r)?,
            scope.lower_any_queries(&queries, r)?,
        ))
    }

    fn define_gate<'c, 'a, F, P, O, B>(&self, backend: &B, gate: &Gate<F>) -> Result<()>
    where
        F: Field,
        P: Default,
        B: Backend<'c, P, O, F = F>,
    {
        let (selectors, queries) = compute_gate_arity(gate.polynomials());
        let scope = backend.define_gate_function(gate.name(), &selectors, &queries)?;

        let resolver = GateScopedResolver {
            selectors: &selectors,
            queries: &queries,
        };
        let stmts = scope.lower_constraints(gate, resolver, "", None);
        backend.lower_stmts(&scope, stmts)?;
        backend.on_scope_end(&scope)
    }
}

impl CodegenStrategy for CallGatesStrat {
    fn codegen<'c, 'a, F, P, O, B>(&self, backend: &B, syn: &CircuitSynthesis<F>) -> Result<()>
    where
        F: Field,
        P: Default,
        B: Backend<'c, P, O, F = F>,
    {
        for gate in syn.gates() {
            self.define_gate(backend, gate)?;
        }

        backend.within_main(syn.advice_io(), syn.instance_io(), |scope| {
            let calls = syn.region_gates().map(|(gate, r)| {
                let (selectors, queries) = compute_gate_arity(gate.polynomials());
                self.create_call_stmt(scope, gate.name(), selectors, queries, &r)
            });
            calls
                .chain(self.inter_region_constraints(scope, syn))
                .collect::<Result<Vec<_>>>()
        })
    }
}

#[derive(Default)]
pub struct InlineConstraintsStrat;

impl CodegenStrategy for InlineConstraintsStrat {
    fn codegen<'c, 'a, F, P, O, B>(&self, backend: &B, syn: &CircuitSynthesis<F>) -> Result<()>
    where
        F: Field,
        P: Default,
        B: Backend<'c, P, O, F = F>,
    {
        backend.within_main(syn.advice_io(), syn.instance_io(), |scope| {
            let lookups = syn.cs().lookups();
            let region = syn
                .regions()
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("No regions"))?;
            let row0 = region.rows().start;
            let region_row = RegionRow::new(region, row0, syn.advice_io(), syn.instance_io());
            for lookup in lookups {
                log::debug!(
                    "lookup {}: exprs {:?} | table {:?}",
                    lookup.name(),
                    lookup.input_expressions(),
                    lookup.table_expressions()
                );
                let lowered_inputs = scope
                    .lower_exprs(
                        lookup.input_expressions().as_ref(),
                        &region_row,
                        &region_row,
                    )
                    .map_err(|err| {
                        log::error!("Failed to lower expressions: {err}");
                        log::error!("Region data: {:?}", region_row);
                        err
                    })?;
                log::debug!("lowered exprs: {:?}", lowered_inputs);
                let lowered_table = scope.lower_exprs(
                    lookup.table_expressions().as_ref(),
                    &region_row,
                    &region_row,
                )?;
                log::debug!("lowered table: {:?}", lowered_table);
            }
            // Do the region stmts first since backends may have more information about names for
            // cells there and some backends do not update the name and always use the first
            // one given.
            syn.region_gates()
                .flat_map(|(gate, r)| {
                    scope.lower_constraints(gate, r, r.region_name(), Some(r.row_number()))
                })
                .chain(self.inter_region_constraints(scope, syn))
                .collect::<Result<Vec<_>>>()
        })
    }
}

pub type WithinMainResult<O> = Result<Vec<CircuitStmt<Value<O>>>>;

pub trait Codegen<'c>: Sized {
    type FuncOutput: Lowering<F = Self::F> + Clone;
    type F: Field + Clone;

    fn within_main<FN>(
        &self,
        advice_io: &CircuitIO<Advice>,
        instance_io: &CircuitIO<Instance>,
        f: FN,
    ) -> Result<()>
    where
        FN: FnOnce(
            &Self::FuncOutput,
        ) -> WithinMainResult<<Self::FuncOutput as Lowering>::CellOutput>,
    {
        let main = self.define_main_function(advice_io, instance_io)?;
        let stmts = f(&main)?;
        self.lower_stmts(&main, stmts.into_iter().map(Ok))?;
        self.on_scope_end(&main)
    }

    fn define_gate_function<'f>(
        &self,
        name: &str,
        selectors: &[&Selector],
        queries: &[AnyQuery],
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f;

    fn define_main_function<'f>(
        &self,
        advice_io: &CircuitIO<Advice>,
        instance_io: &CircuitIO<Instance>,
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f;

    fn lower_stmts(
        &self,
        scope: &Self::FuncOutput,
        stmts: impl Iterator<
            Item = Result<CircuitStmt<Value<<Self::FuncOutput as Lowering>::CellOutput>>>,
        >,
    ) -> Result<()> {
        lower_stmts(scope, stmts)
    }

    fn on_scope_end(&self, _: &Self::FuncOutput) -> Result<()> {
        Ok(())
    }
}

fn lower_stmts<Scope: Lowering>(
    scope: &Scope,
    stmts: impl Iterator<Item = Result<CircuitStmt<Value<<Scope as Lowering>::CellOutput>>>>,
) -> Result<()> {
    for stmt in stmts {
        let stmt = stmt?;
        match stmt {
            CircuitStmt::ConstraintCall(name, selectors, queries) => {
                scope.generate_call(&name, &selectors, &queries)?;
            }
            CircuitStmt::Constraint(op, lhs, rhs) => {
                scope.checked_generate_constraint(op, &lhs, &rhs)?;
            }
            CircuitStmt::Comment(s) => scope.generate_comment(s)?,
        };
    }
    Ok(())
}

pub trait Backend<'c, Params: Default, Output>: Codegen<'c> {
    fn initialize(params: Params) -> Self;

    fn generate_output(self) -> Result<Output>;

    /// Generate code using the given strategy.
    fn codegen<C>(self, circuit: &C) -> Result<Output>
    where
        C: CircuitWithIO<Self::F>,
    {
        self.codegen_with_strat::<C, InlineConstraintsStrat>(circuit)
    }

    /// Generate code using the given strategy.
    fn codegen_with_strat<C, S>(self, circuit: &C) -> Result<Output>
    where
        C: CircuitWithIO<Self::F>,
        S: CodegenStrategy,
    {
        let syn = CircuitSynthesis::new(circuit)?;

        S::default().codegen(&self, &syn)?;

        self.generate_output()
    }
}
