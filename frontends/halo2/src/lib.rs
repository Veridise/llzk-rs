use crate::halo2::{
    Advice, AdviceQuery, Any, Column, ColumnType, Field, FixedQuery, Gate, Instance, InstanceQuery,
    Rotation, Selector,
};
use anyhow::{bail, Result};
use backend::{lowering::Lowering, Backend};
use gates::compute_gate_arity;
use ir::CircuitStmt;
use synthesis::regions::{RegionRow, Row};
use synthesis::CircuitSynthesis;

pub(crate) mod backend;
mod gates;
mod halo2;
mod io;
mod ir;
mod synthesis;
#[cfg(test)]
mod test;
mod value;

pub use io::{CircuitIO, CircuitWithIO};

//pub type Output = <LLZKBackend as Backend>::Output;
//pub type PicusOutput = <PicusBackend as Backend>::Output;

//pub fn codegen<F: Field, C: CircuitWithIO<F>>(circuit: &C) -> Result<Output> {
//    codegen_impl::<F, C, LLZKBackend>(circuit)
//}
//
//pub fn picus_codegen<F: Field, C: CircuitWithIO<F>>(circuit: &C) -> Result<PicusOutput> {
//    codegen_impl::<F, C, PicusBackend>(circuit)
//}
