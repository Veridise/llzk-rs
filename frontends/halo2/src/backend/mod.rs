use crate::{
    gates::{compute_gate_arity, AnyQuery},
    halo2::{
        Advice, AdviceQuery, Any, Column, Expression, Field, FixedQuery, Gate, Instance,
        InstanceQuery, Rotation, Selector, Value,
    },
    ir::{CircuitStmt, CircuitStmts},
    synthesis::{
        regions::{RegionRow, Row},
        CircuitSynthesis,
    },
    CircuitIO, CircuitWithIO,
};
use anyhow::{anyhow, Result};

pub mod func;
pub mod llzk;
pub mod lowering;
pub mod picus;
pub mod resolvers;

use func::ArgNo;
use lowering::Lowering;
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

    fn resolve_advice_query(&self, query: &AdviceQuery) -> Result<ResolvedQuery<F>> {
        resolve(
            self.queries.iter(),
            query,
            self.selectors.len(),
            "Query as argument not found",
        )
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
            self.selectors.iter().map(|s| *s),
            selector,
            0,
            "Selector as argument not found",
        )
    }
}

pub trait Backend<'c, Params: Default, Output>: Sized {
    type FuncOutput: Lowering< F = Self::F> + Clone;
    type F: Field;

    fn initialize(params: Params) -> Self;

    fn generate_output(&'c self) -> Result<Output>;

    fn define_gate_function<'f>(
        &'c self,
        name: &str,
        selectors: &[&Selector],
        queries: &[AnyQuery],
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f;

    fn define_main_function<'f>(
        &'c self,
        advice_io: &CircuitIO<Advice>,
        instance_io: &CircuitIO<Instance>,
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f;

    fn define_gate(&'c self, gate: &Gate<Self::F>) -> Result<()> {
        let (selectors, queries) = compute_gate_arity(gate.polynomials());
        let scope = self.define_gate_function(gate.name(), &selectors, &queries)?;

        let resolver = GateScopedResolver {
            selectors: &selectors,
            queries: &queries,
        };
        let exprs = scope.lower_exprs(gate.polynomials(), &resolver, &resolver)?;
        let zero = scope.lower_expr(&Expression::Constant(Self::F::ZERO), &resolver, &resolver)?;
        for expr in exprs {
            scope.checked_generate_constraint(&expr, &zero)?;
        }

        Ok(())
    }

    fn within_main<FN>(
        &'c self,
        advice_io: &CircuitIO<Advice>,
        instance_io: &CircuitIO<Instance>,
        f: FN,
    ) -> Result<()>
    where
        FN: FnOnce(
            &Self::FuncOutput,
        ) -> Result<CircuitStmts<<Self::FuncOutput as Lowering>::CellOutput>>,
    {
        let main = self.define_main_function(advice_io, instance_io)?;
        for stmt in f(&main)? {
            match stmt {
                CircuitStmt::ConstraintCall(name, selectors, queries) => {
                    main.generate_call(&name, &selectors, &queries)?;
                }
                CircuitStmt::EqConstraint(lhs, rhs) => {
                    main.checked_generate_constraint(&lhs, &rhs)?;
                }
            };
        }

        Ok(())
    }

    /// Generate code using the given backend. This function is made pub(crate) to give the option of
    /// injecting mock backends for testing.
    fn codegen_impl<C>(&'c self, circuit: &C) -> Result<Output>
    where
        C: CircuitWithIO<Self::F>,
    {
        let syn = CircuitSynthesis::new(circuit)?;
        for gate in syn.gates() {
            self.define_gate(gate)?;
        }

        self.within_main(syn.advice_io(), syn.instance_io(), |scope| {
            let create_call_stmt = |name: &str,
                                    selectors: Vec<&Selector>,
                                    queries: Vec<AnyQuery>,
                                    r: &RegionRow<Self::F>| -> Result<CircuitStmt<<Self::FuncOutput as Lowering>::CellOutput>> {
                Ok(CircuitStmt::ConstraintCall(
                    name.to_owned(),
                    scope.lower_selectors(&selectors, r)?,
                    scope.lower_any_queries(&queries, r)?,
                ))
            };

            let lower_gate_call = |gate: &Gate<Self::F>, r: &RegionRow<Self::F>| {
                let (selectors, queries) = compute_gate_arity(gate.polynomials());
                if r.gate_is_disabled(&selectors) {
                    return None;
                }

                Some(create_call_stmt(gate.name(), selectors, queries, r))
            };

            let lower_cell =
                |(col, row): &(Column<Any>, usize)| -> Result<Value<<Self::FuncOutput as Lowering>::CellOutput>> {
                    let q = col.query_cell::<Self::F>(Rotation::cur());
                    let row = Row::new(*row, syn.advice_io(), syn.instance_io());
                    scope.lower_expr(&q, &row, &row)
                };

            

            let calls = syn
                .regions()
                .iter()
                .flat_map(|region| {
                    region
                        .rows()
                        .map(|row| {
                            RegionRow::new(region, row, syn.advice_io(), syn.instance_io())})
                })
                .flat_map(|r| {
                    syn.gates()
                        .iter()
                        .filter_map(move |gate| lower_gate_call(gate, &r))
                });

            let mut constraints = syn.constraints().collect::<Vec<_>>();
    constraints.sort();
            let constraints = constraints.into_iter().map(|(from, to)| {
                    Ok(CircuitStmt::EqConstraint(
                        lower_cell(from)?,
                        lower_cell(to)?,
                    ))
                });
            
            calls.chain(constraints)
                .collect::<Result<Vec<_>>>()
        })?;
        // General idea:
        //  - For each gate generate modules that execute the constraints defined by their polynomials.
        //  - Then, create a '@Main' struct that has the rest of the circuit's logic.
        //      - Each input advice is an argument of the struct.
        //      - Each output advice is a field of the struct.
        //      - Each input instance is an argument of the struct with the pub attribute.
        //      - Each output instance is a field of the struct with the pub attribute.
        //      - Every advice that is neither an input or an output needs to be handled differently.
        //          - Fields? Probably the easiest way but they then get mixed up with the outputs.
        //          - Mark the others as columns to differentiate?
        //      - For each region find what selectors are enabled and create calls to the gate's module
        //  with the required arguments.
        //      - For each permutation pair create a constraint between the two elements.
        //
        self.generate_output()
    }
}
