use crate::ir::expr::Felt;

use super::vars::NamingConvention;

#[derive(Clone)]
pub struct PicusParams {
    expr_cutoff: Option<usize>,
    entrypoint: String,
    #[cfg(feature = "lift-field-operations")]
    lift_fixed: bool,
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

    #[cfg(feature = "lift-field-operations")]
    pub fn lift_fixed(&self) -> bool {
        self.lift_fixed
    }

    pub fn prime(&self) -> &picus::felt::Felt {
        &self.prime
    }

    fn new<F: crate::halo2::PrimeField>() -> Self {
        Self {
            prime: Felt::prime::<F>().into(),
            expr_cutoff: None,
            entrypoint: "Main".to_owned(),
            #[cfg(feature = "lift-field-operations")]
            lift_fixed: false,
            naming_convention: NamingConvention::Default,
            optimize: true,
        }
    }
}

#[cfg(feature = "lift-field-operations")]
impl crate::ir::lift::LiftingCfg for PicusParams {
    fn lifting_enabled(&self) -> bool {
        self.lift_fixed()
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

    #[cfg(feature = "lift-field-operations")]
    pub fn no_lift_fixed(self) -> Self {
        let mut p = self.0;
        p.lift_fixed = false;
        Self(p)
    }

    #[cfg(feature = "lift-field-operations")]
    pub fn lift_fixed(self) -> Self {
        let mut p = self.0;
        p.lift_fixed = true;
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
