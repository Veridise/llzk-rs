use super::GateScopedResolver;
use crate::backend::codegen::{
    inter_region_constraints, lower_constraints, Codegen, CodegenStrategy,
};
use crate::backend::lowering::{Lowerable, Lowering};
use crate::expressions::ScopedExpression;
use crate::{
    gates::{compute_gate_arity, AnyQuery},
    halo2::{Gate, Selector},
    ir::CircuitStmt,
    synthesis::{regions::RegionRow, CircuitSynthesis},
};
use anyhow::Result;

macro_rules! create_call_stmt {
    ($name:expr,$selectors:expr,$queries:expr,$r:expr) => {{
        let inputs = $selectors
            .into_iter()
            .map(|s| ScopedExpression::new(s.expr(), $r))
            .chain(
                $queries
                    .into_iter()
                    .map(|q| ScopedExpression::new(q.expr(), $r)),
            );
        CircuitStmt::call($name.to_owned(), inputs, vec![])
    }};
}

#[derive(Default)]
pub struct CallGatesStrat;

impl CallGatesStrat {
    fn define_gate<'c, 'C>(
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
            selectors,
            queries,
            outputs: Default::default(),
        };
        let stmts = lower_constraints(gate, resolver, "<no region>", None);
        backend.lower_stmts(&scope, stmts.map(Ok))?;
        backend.on_scope_end(scope)
    }
}

impl CodegenStrategy for CallGatesStrat {
    fn codegen<'c, 's, C>(&self, backend: &C, syn: &'s CircuitSynthesis<C::F>) -> Result<()>
    where
        C: Codegen<'c>,
    {
        for gate in syn.gates() {
            self.define_gate(backend, gate, syn)?;
        }

        backend.within_main(syn, move |_| {
            let calls = syn.region_gates().map(|(gate, r)| {
                let (selectors, queries) = compute_gate_arity(gate.polynomials());
                let inputs = selectors
                    .into_iter()
                    .map(|s| ScopedExpression::<'_, 's, _>::new(s.expr(), Box::new(r)))
                    .chain(
                        queries
                            .into_iter()
                            .map(|q| ScopedExpression::new(q.expr(), Box::new(r))),
                    );
                CircuitStmt::call(gate.name().to_owned(), inputs, vec![])
            });
            Ok(calls
                .chain(inter_region_constraints(syn)?)
                .collect::<Vec<_>>())
        })
    }
}
