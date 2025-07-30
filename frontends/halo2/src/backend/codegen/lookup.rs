use crate::backend::func::FuncIO;
use crate::backend::lowering::Lowering;
use crate::backend::resolvers::{QueryResolver, ResolvedQuery};
use crate::{
    gates::{compute_gate_arity, AnyQuery},
    halo2::{Expression, Field, Selector},
    ir::CircuitStmt,
    synthesis::{regions::RegionRow, CircuitSynthesis},
};
use anyhow::{anyhow, Result};

use super::strats::GateScopedResolver;
use super::Codegen;

pub mod codegen;

#[derive(Clone)]
pub struct Lookup<'a, F: Field> {
    name: &'a str,
    id: usize,
    inputs: &'a [Expression<F>],
    table_expressions: &'a [Expression<F>],
    selectors: Vec<&'a Selector>,
    queries: Vec<AnyQuery>,
    table: Vec<AnyQuery>,
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
            .enumerate()
            .map(|(id, a)| {
                let inputs = a.input_expressions();
                let (selectors, queries) = compute_gate_arity(inputs);
                let table = compute_table_cells(a.table_expressions().iter())?;
                Ok(Self {
                    name: a.name(),
                    id,
                    inputs,
                    table_expressions: a.table_expressions(),
                    selectors,
                    queries,
                    table,
                })
            })
            .collect()
    }

    fn module_name(&self) -> String {
        format!("lookup{}_{}", self.id, self.name)
    }

    pub fn create_scope<'c, C>(
        &self,
        backend: &C,
        syn: &CircuitSynthesis<C::F>,
    ) -> Result<C::FuncOutput>
    where
        C: Codegen<'c>,
    {
        backend.define_gate_function(
            &self.module_name(),
            &self.selectors,
            &self.queries,
            self.output_queries(),
            syn,
        )
    }

    pub fn output_queries(&self) -> &[AnyQuery] {
        &self.table
    }

    pub fn create_resolver(&self) -> GateScopedResolver {
        GateScopedResolver {
            selectors: &self.selectors,
            queries: &self.queries,
            outputs: &self.table,
        }
    }

    pub fn expressions(&self) -> impl Iterator<Item = (&Expression<F>, &Expression<F>)> {
        self.inputs.iter().zip(self.table_expressions)
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

        let resolved = self.resolve_table_column_queries(r.row_number()).collect();

        Ok(CircuitStmt::ConstraintCall(
            self.module_name(),
            inputs,
            resolved,
        ))
    }

    fn resolve_table_column_queries(&self, row: usize) -> impl Iterator<Item = FuncIO> {
        self.table
            .iter()
            .inspect(|q| log::debug!("Table query: {q:?}"))
            .map(move |q| FuncIO::TableLookup(self.id, q.column_index(), row))
    }
}
