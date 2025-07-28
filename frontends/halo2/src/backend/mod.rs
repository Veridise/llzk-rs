use std::borrow::Cow;

use crate::{
    gates::{compute_gate_arity, AnyQuery},
    halo2::{
        AdviceQuery, Any, Column, Field, FixedQuery, Gate, InstanceQuery, Rotation, Selector, Value,
    },
    ir::{BinaryBoolOp, CircuitStmt},
    synthesis::{
        regions::{RegionRow, Row, FQN},
        CircuitSynthesis,
    },
    CircuitWithIO,
};
use anyhow::{anyhow, Result};

pub mod events;
pub mod func;
pub mod llzk;
pub mod lowering;
pub mod picus;
pub mod resolvers;

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

    fn resolve_advice_query(
        &self,
        query: &AdviceQuery,
    ) -> Result<(ResolvedQuery<F>, Option<Cow<FQN>>)> {
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
    ) -> impl Iterator<Item = Result<CircuitStmt<L::CellOutput>>> + 'c
    where
        F: Field,
        L: Lowering<F = F>,
    {
        let lower_cell = |(col, row): &(Column<Any>, usize)| -> Result<L::CellOutput> {
            let q = col.query_cell::<L::F>(Rotation::cur());
            let row = Row::new(*row, syn.regions_ref(), syn.advice_io(), syn.instance_io());
            scope.lower_expr(&q, &row, &row)
        };
        let mut constraints = syn.constraints().collect::<Vec<_>>();
        constraints.sort();
        constraints.into_iter().map(move |(from, to)| {
            log::debug!("{from:?} == {to:?}");
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
    ) -> Result<CircuitStmt<L::CellOutput>>
    where
        F: Field,
        L: Lowering<F = F>,
    {
        let mut inputs = scope.lower_selectors(&selectors, r)?;
        inputs.extend(scope.lower_any_queries(&queries, r)?);
        Ok(CircuitStmt::ConstraintCall(name.to_owned(), inputs, vec![]))
    }

    fn define_gate<'c, 'a, F, P, O, B>(
        &self,
        backend: &B,
        gate: &Gate<F>,
        syn: &CircuitSynthesis<F>,
    ) -> Result<()>
    where
        F: Field,
        P: Default,
        B: Backend<'c, P, O, F = F>,
    {
        let (selectors, queries) = compute_gate_arity(gate.polynomials());
        let scope = backend.define_gate_function(gate.name(), &selectors, &queries, &[], syn)?;

        let resolver = GateScopedResolver {
            selectors: &selectors,
            queries: &queries,
        };
        let stmts = scope.lower_constraints(gate, resolver, "<no region>", None);
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
            self.define_gate(backend, gate, syn)?;
        }

        backend.within_main(syn, |scope| {
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

#[derive(Clone)]
struct Lookup<'a, F: Field> {
    name: &'a str,
    inputs: &'a [Expression<F>],
    table_expressions: &'a [Expression<F>],
    selectors: Vec<&'a Selector>,
    queries: Vec<AnyQuery>,
    table: Vec<AnyQuery>,
    all_queries: Vec<AnyQuery>,
}

fn compute_table_cells<'a, F: Field>(
    table: impl Iterator<Item = &'a Expression<F>>,
) -> Result<Vec<AnyQuery>> {
    table
        .map(|e| match e {
            Expression::Fixed(fixed_query) => Ok(fixed_query.into()),
            _ => Err(anyhow!(
                "Table row expressions can only be fixed cell queries"
            )),
        })
        .collect()
}

impl<'a, F: Field> Lookup<'a, F> {
    pub fn load(syn: &'a CircuitSynthesis<F>) -> Result<Vec<Self>> {
        syn.cs()
            .lookups()
            .iter()
            .map(|a| {
                let inputs = a.input_expressions();
                let (selectors, queries) = compute_gate_arity(&inputs);
                let table = compute_table_cells(a.table_expressions().into_iter())?;
                let mut all_queries = queries.clone();
                all_queries.extend(table.clone());
                Ok(Self {
                    name: a.name(),
                    inputs: &inputs,
                    table_expressions: &a.table_expressions(),
                    selectors,
                    queries,
                    table,
                    all_queries,
                })
            })
            .collect()
    }

    pub fn create_scope<'c, P, O, B>(
        &self,
        backend: &B,
        syn: &CircuitSynthesis<F>,
    ) -> Result<B::FuncOutput>
    where
        F: Field,
        P: Default,
        B: Backend<'c, P, O, F = F>,
    {
        backend.define_gate_function(self.name, &self.selectors, &self.queries, &self.table, syn)
    }

    pub fn create_resolver(&self) -> GateScopedResolver {
        GateScopedResolver {
            selectors: &self.selectors,
            queries: &self.all_queries,
        }
    }

    pub fn expressions(&self) -> impl Iterator<Item = (&Expression<F>, &Expression<F>)> {
        self.inputs.into_iter().zip(self.table_expressions)
    }

    pub fn comment_header<T>(&self) -> CircuitStmt<T> {
        CircuitStmt::Comment(format!("lookup '{}'", self.name))
    }

    pub fn create_call_stmt<L>(
        &self,
        scope: &L,
        r: &RegionRow<F>,
    ) -> Result<CircuitStmt<L::CellOutput>>
    where
        F: Field,
        L: Lowering<F = F>,
    {
        let mut inputs = scope.lower_selectors(&self.selectors, r)?;
        inputs.extend(scope.lower_any_queries(&self.queries, r)?);
        let resolved = self
            .table
            .iter()
            .map(|o| r.resolve_any_query(o))
            .map(|r| match r? {
                ResolvedQuery::Lit(_) => Err(anyhow!(
                    "Fixed table columns cannot have an assigned fixed value"
                )),
                ResolvedQuery::IO(func_io) => Ok(func_io),
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(CircuitStmt::ConstraintCall(
            self.name.to_owned(),
            inputs,
            resolved,
        ))
    }
}

#[derive(Default)]
pub struct InlineConstraintsStrat;

impl InlineConstraintsStrat {
    fn compute_table_cells<'a, F: Field>(
        &self,
        table: impl Iterator<Item = &'a Expression<F>>,
    ) -> Result<Vec<AnyQuery>> {
        table
            .map(|e| match e {
                Expression::Fixed(fixed_query) => Ok(fixed_query.into()),
                _ => Err(anyhow!(
                    "Table row expressions can only be fixed cell queries"
                )),
            })
            .collect()
    }

    fn define_lookup_modules<'c, 'a, F, P, O, B>(
        &self,
        backend: &B,
        syn: &CircuitSynthesis<F>,
    ) -> Result<()>
    where
        F: Field,
        P: Default,
        B: Backend<'c, P, O, F = F>,
    {
        for lookup in Lookup::load(syn)? {
            let scope = lookup.create_scope(backend, syn)?;
            let resolver = lookup.create_resolver();
            let constraints = lookup.expressions().map(|(input, table)| {
                Ok(CircuitStmt::Constraint(
                    BinaryBoolOp::Eq,
                    scope.lower_expr(input, &resolver, &resolver)?,
                    scope.lower_expr(table, &resolver, &resolver)?,
                ))
            });

            let stmts = vec![Ok(lookup.comment_header())]
                .into_iter()
                .chain(constraints);

            // TODO: Missing the assume-determinisitic statements.
            backend.lower_stmts(&scope, stmts)?;
            backend.on_scope_end(&scope)?;
        }
        Ok(())
    }
}

impl CodegenStrategy for InlineConstraintsStrat {
    fn codegen<'c, 'a, F, P, O, B>(&self, backend: &B, syn: &CircuitSynthesis<F>) -> Result<()>
    where
        F: Field,
        P: Default,
        B: Backend<'c, P, O, F = F>,
    {
        self.define_lookup_modules(backend, syn)?;

        backend.within_main(syn, |scope| {
            let region_rows = || {
                syn.regions().into_iter().flat_map(|r| {
                    r.rows().map(move |row| {
                        RegionRow::new(
                            r,
                            row,
                            syn.regions_ref(),
                            syn.advice_io(),
                            syn.instance_io(),
                        )
                    })
                })
            };
            let lookups = Lookup::load(syn)?
                .into_iter()
                .flat_map(|l| region_rows().map(move |r| l.create_call_stmt(scope, &r)));
            // Do the region stmts first since backends may have more information about names for
            // cells there and some backends do not update the name and always use the first
            // one given.
            syn.region_gates()
                .flat_map(|(gate, r)| {
                    scope.lower_constraints(gate, r, r.header(), Some(r.row_number()))
                })
                .chain(lookups)
                .chain(self.inter_region_constraints(scope, syn))
                .collect::<Result<Vec<_>>>()
        })
    }
}

pub type WithinMainResult<O> = Result<Vec<CircuitStmt<O>>>;

pub trait Codegen<'c>: Sized {
    type FuncOutput: Lowering<F = Self::F> + Clone;
    type F: Field + Clone;

    fn within_main<FN>(&self, syn: &CircuitSynthesis<Self::F>, f: FN) -> Result<()>
    where
        FN: FnOnce(
            &Self::FuncOutput,
        ) -> WithinMainResult<<Self::FuncOutput as Lowering>::CellOutput>,
    {
        let main = self.define_main_function(syn)?;
        let stmts = f(&main)?;
        self.lower_stmts(&main, stmts.into_iter().map(Ok))?;
        self.on_scope_end(&main)
    }

    fn define_gate_function<'f>(
        &self,
        name: &str,
        selectors: &[&Selector],
        input_queries: &[AnyQuery],
        output_queries: &[AnyQuery],
        syn: &CircuitSynthesis<Self::F>,
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f;

    fn define_main_function<'f>(&self, syn: &CircuitSynthesis<Self::F>) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f;

    fn lower_stmts(
        &self,
        scope: &Self::FuncOutput,
        stmts: impl Iterator<Item = Result<CircuitStmt<<Self::FuncOutput as Lowering>::CellOutput>>>,
    ) -> Result<()> {
        lower_stmts(scope, stmts)
    }

    fn on_scope_end(&self, _: &Self::FuncOutput) -> Result<()> {
        Ok(())
    }
}

fn lower_stmts<Scope: Lowering>(
    scope: &Scope,
    stmts: impl Iterator<Item = Result<CircuitStmt<<Scope as Lowering>::CellOutput>>>,
) -> Result<()> {
    for stmt in stmts {
        let stmt = stmt?;
        match stmt {
            CircuitStmt::ConstraintCall(name, inputs, outputs) => {
                scope.generate_call(&name, &inputs, &outputs)?;
            }
            CircuitStmt::Constraint(op, lhs, rhs) => {
                scope.checked_generate_constraint(op, &lhs, &rhs)?;
            }
            CircuitStmt::Comment(s) => scope.generate_comment(s)?,
            CircuitStmt::AssumeDeterministic(func_io) => {
                scope.generate_assume_deterministic(func_io)?
            }
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
