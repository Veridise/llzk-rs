
use super::GateScopedResolver;
use crate::backend::codegen::{Codegen, CodegenStrategy};
use crate::backend::lowering::Lowering;
use crate::{
    gates::{compute_gate_arity, AnyQuery},
    halo2::{
        Gate, Selector,
    },
    ir::CircuitStmt,
    synthesis::{
        regions::RegionRow,
        CircuitSynthesis,
    },
};
use anyhow::Result;

#[derive(Default)]
pub struct CallGatesStrat;

impl CallGatesStrat {
    fn create_call_stmt<L>(
        &self,
        scope: &L,
        name: &str,
        selectors: Vec<&Selector>,
        queries: Vec<AnyQuery>,
        r: &RegionRow<L::F>,
    ) -> Result<CircuitStmt<L::CellOutput>>
    where
        L: Lowering,
    {
        let mut inputs = scope.lower_selectors(&selectors, r)?;
        inputs.extend(scope.lower_any_queries(&queries, r)?);
        Ok(CircuitStmt::ConstraintCall(name.to_owned(), inputs, vec![]))
    }

    fn define_gate<'c, C>(
        &self,
        backend: &C,
        gate: &Gate<C::F>,
        syn: &CircuitSynthesis<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c>,
    {
        let (selectors, queries) = compute_gate_arity(gate.polynomials());
        let scope = backend.define_gate_function(gate.name(), &selectors, &queries, &[], syn)?;

        let resolver = GateScopedResolver {
            selectors: &selectors,
            queries: &queries,
            outputs: &[],
        };
        let stmts = scope.lower_constraints(gate, resolver, "<no region>", None);
        backend.lower_stmts(&scope, stmts)?;
        backend.on_scope_end(scope)
    }
}

impl CodegenStrategy for CallGatesStrat {
    fn codegen<'c, C>(&self, backend: &C, syn: &CircuitSynthesis<C::F>) -> Result<()>
    where
        C: Codegen<'c>,
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
