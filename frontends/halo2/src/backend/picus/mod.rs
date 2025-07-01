use std::{cell::RefCell, marker::PhantomData};

use super::{func::FuncIO, Backend};
use crate::{
    gates::AnyQuery,
    halo2::{Advice, Instance, PrimeField, Selector},
    CircuitIO, LiftLike,
};
use anyhow::Result;

mod lowering;
mod vars;

pub use lowering::PicusModuleLowering;
use lowering::PicusModuleRef;
use num_bigint::BigUint;
use picus::{
    felt::{Felt, IntoPrime},
    ModuleLike as _,
};
use vars::VarKey;

pub type PicusModule = picus::Module<VarKey>;
pub type PicusOutput<F> = picus::Program<FeltWrap<F>, VarKey>;

pub struct PicusParams {
    expr_cutoff: usize,
    entrypoint: String,
    lift_fixed: bool,
}

impl PicusParams {
    pub fn builder() -> PicusParamsBuilder {
        PicusParamsBuilder(Default::default())
    }
}

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

impl<F: PrimeField> Into<Felt> for FeltWrap<F> {
    fn into(self) -> Felt {
        let r = self.0.to_repr();
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
}

impl Into<PicusParams> for PicusParamsBuilder {
    fn into(self) -> PicusParams {
        self.0
    }
}

impl Default for PicusParams {
    fn default() -> Self {
        Self {
            expr_cutoff: 10,
            entrypoint: "Main".to_owned(),
            lift_fixed: false,
        }
    }
}

pub struct PicusBackend<F, L> {
    params: PicusParams,
    modules: RefCell<Vec<PicusModuleRef>>,
    _marker: PhantomData<(F, L)>,
}

fn mk_io<F, I, O>(count: usize, f: F) -> impl Iterator<Item = O>
where
    O: Into<VarKey>,
    I: From<usize>,
    F: Fn(I) -> O + 'static,
{
    (0..count).map(move |i| f(i.into()))
}

impl<'c, F: PrimeField, L: LiftLike<Inner = F>> Backend<'c, PicusParams, PicusOutput<L>>
    for PicusBackend<F, L>
{
    type FuncOutput = PicusModuleLowering<F, L>;
    type F = L;

    fn initialize(params: PicusParams) -> Self {
        Self {
            params,
            modules: Default::default(),
            _marker: Default::default(),
        }
    }

    fn generate_output(&'c self) -> Result<PicusOutput<Self::F>> {
        let mut output = PicusOutput::from(self.modules.borrow().clone());

        for module in output.modules_mut() {
            module.fold_stmts();
        }
        // TODO: Cut the expressions that are too big
        Ok(output)
    }

    fn define_gate_function<'f>(
        &'c self,
        name: &str,
        selectors: &[&Selector],
        queries: &[AnyQuery],
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        let module = PicusModule::shared(
            name.to_owned(),
            mk_io(selectors.len() + queries.len(), FuncIO::Arg),
            mk_io(0, FuncIO::Field),
        );
        self.modules.borrow_mut().push(module.clone());
        Ok(Self::FuncOutput::new(module, self.params.lift_fixed))
    }

    fn define_main_function<'f>(
        &'c self,
        advice_io: &CircuitIO<Advice>,
        instance_io: &CircuitIO<Instance>,
    ) -> Result<Self::FuncOutput>
    where
        Self::FuncOutput: 'f,
        'c: 'f,
    {
        let module = PicusModule::shared(
            self.params.entrypoint.clone(),
            mk_io(
                instance_io.inputs().len() + advice_io.inputs().len(),
                FuncIO::Arg,
            ),
            mk_io(
                instance_io.outputs().len() + advice_io.outputs().len(),
                FuncIO::Field,
            ),
        );
        self.modules.borrow_mut().push(module.clone());
        Ok(Self::FuncOutput::new(module, self.params.lift_fixed))
    }
}
