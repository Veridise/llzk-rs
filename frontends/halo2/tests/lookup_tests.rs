#[cfg(feature = "picus-backend")]
use common::picus::basic_picus_test;
use common::synthesis_impl;
use group::ff::Field;
use halo2_llzk_frontend::{
    LookupCallbacks,
    ir::generate::IRGenParamsBuilder,
    lookups::{Lookup, table::LookupTableGenerator},
    temps::{ExprOrTemp, Temps},
};
use halo2_midnight_integration::plonk::_Expression;
use halo2_test_circuits::lookup;
use halo2curves::bn256::Fr;
use haloumi_ir::stmt::IRStmt;
use std::borrow::Cow;

mod common;

basic_picus_test! {
    lookup_circuit,
    LookupCircuitSynthesis::default(),
    include_str!("expected/picus/lookup.picus"),
    include_str!("expected/picus/lookup_opt.picus"),
    IRGenParamsBuilder::new()
                        .lookup_callbacks(&LookupCallbackHandler)
                        .build()
}

basic_picus_test! {
    lookup_2x3_circuit,
    Lookup2x3CircuitSynthesis::default(),
    include_str!("expected/picus/lookup_2x3.picus"),
    include_str!("expected/picus/lookup_2x3_opt.picus"),
    IRGenParamsBuilder::new()
                        .lookup_callbacks(&LookupCallbackHandler)
                        .build(),
}

basic_picus_test! {
    lookup_2x3_fixed_circuit,
    Lookup2x3FixedCircuitSynthesis::default(),
    include_str!("expected/picus/lookup_2x3.picus"),
    include_str!("expected/picus/lookup_2x3_opt.picus"),
    IRGenParamsBuilder::new()
                        .lookup_callbacks(&LookupCallbackHandler)
                        .build(),
}

basic_picus_test! {
    lookup_2x3_zerosel_circuit,
    Lookup2x3ZeroSelCircuitSynthesis::default(),
    include_str!("expected/picus/lookup_2x3.picus"),
    include_str!("expected/picus/lookup_2x3_opt.picus"),
    IRGenParamsBuilder::new()
                        .lookup_callbacks(&LookupCallbackHandler)
                        .build(),
}

synthesis_impl!(LookupCircuitSynthesis, lookup::LookupCircuit<Fr>, [0], [1]);
synthesis_impl!(
    Lookup2x3CircuitSynthesis,
    lookup::two_by_three::Lookup2x3Circuit<Fr>,
    [0],
    [1]
);
synthesis_impl!(
    Lookup2x3FixedCircuitSynthesis,
    lookup::two_by_three_fixed::Lookup2x3Circuit<Fr>,
    [0],
    [1]
);
synthesis_impl!(
    Lookup2x3ZeroSelCircuitSynthesis,
    lookup::two_by_three_zerosel::Lookup2x3ZeroSelCircuit<Fr>,
    [0],
    [1]
);

struct LookupCallbackHandler;

impl<F: Field> LookupCallbacks<F, _Expression<F>> for LookupCallbackHandler {
    fn on_lookup<'a>(
        &self,
        _lookup: &'a Lookup<_Expression<F>>,
        _table: &dyn LookupTableGenerator<F>,
        _temps: &mut Temps,
    ) -> anyhow::Result<IRStmt<ExprOrTemp<Cow<'a, _Expression<F>>>>> {
        Ok(IRStmt::comment("Ignored lookup"))
    }
}
