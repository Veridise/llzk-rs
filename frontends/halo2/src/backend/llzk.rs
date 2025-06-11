//use super::{Backend, GateDefinition};
//use crate::halo2::{Field, Gate};
//use anyhow::Result;
//
//pub struct LLZKBackend {}
//
//pub struct GateDef;
//
//impl GateDefinition for GateDef {}
//
//impl Backend for LLZKBackend {
//    type Context = ();
//    type Output = ();
//    type GD = GateDef;
//
//    fn initialize() -> Self::Context {
//        todo!()
//    }
//
//    fn define_gate<F: Field>(_ctx: &Self::Context, _gate: &Gate<F>) -> Self::GD {
//        todo!()
//    }
//
//    fn generate_output(_ctx: Self::Context) -> Result<Self::Output> {
//        todo!()
//    }
//}
