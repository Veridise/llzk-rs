use crate::ir::expr::Felt;

use super::vars::NamingConvention;

#[derive(Clone)]
pub struct PicusParams {
    expr_cutoff: Option<usize>,
    entrypoint: String,
    naming_convention: NamingConvention,
    optimize: bool,
    prime: picus::felt::Felt,
}

impl PicusParams {
    pub fn naming_convention(&self) -> NamingConvention {
        self.naming_convention
    }

    pub fn optimize(&self) -> bool {
        self.optimize
    }

    pub fn expr_cutoff(&self) -> Option<usize> {
        self.expr_cutoff
    }

    pub fn entrypoint(&self) -> &str {
        &self.entrypoint
    }

    pub fn prime(&self) -> &picus::felt::Felt {
        &self.prime
    }

    fn new<F: crate::halo2::PrimeField>() -> Self {
        Self {
            prime: Felt::prime::<F>().into(),
            expr_cutoff: None,
            entrypoint: "Main".to_owned(),
            naming_convention: NamingConvention::Short,
            optimize: true,
        }
    }
}

pub struct PicusParamsBuilder(PicusParams);

impl PicusParamsBuilder {
    pub fn new<F: crate::halo2::PrimeField>() -> Self {
        Self(PicusParams::new::<F>())
    }

    pub fn expr_cutoff(self, expr_cutoff: usize) -> Self {
        let mut p = self.0;
        p.expr_cutoff = Some(expr_cutoff);
        Self(p)
    }

    pub fn no_expr_cutoff(self) -> Self {
        let mut p = self.0;
        p.expr_cutoff = None;
        Self(p)
    }

    pub fn entrypoint(self, name: &str) -> Self {
        let mut p = self.0;
        p.entrypoint = name.to_owned();
        Self(p)
    }

    pub fn short_names(mut self) -> Self {
        self.0.naming_convention = NamingConvention::Short;
        self
    }

    pub fn optimize(mut self) -> Self {
        self.0.optimize = true;
        self
    }

    pub fn no_optimize(mut self) -> Self {
        self.0.optimize = false;
        self
    }

    pub fn build(self) -> PicusParams {
        self.0
    }
}

impl From<PicusParamsBuilder> for PicusParams {
    fn from(builder: PicusParamsBuilder) -> PicusParams {
        builder.0
    }
}
