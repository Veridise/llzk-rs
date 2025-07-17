use super::{
    vars::{NamingConvention, VarKey, VarKeySeed},
    FeltWrap,
};
use crate::{
    backend::{
        lowering::Lowering,
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    halo2::{
        AdviceQuery, Challenge, FixedQuery, InstanceQuery, RegionIndex, RegionStart, Selector,
        Value,
    },
    ir::{lift::LiftLowering, BinaryBoolOp},
    synthesis::regions::FQN,
    value::{steal, steal_many},
    LiftLike,
};
use anyhow::{anyhow, bail, Result};
use picus::{expr, stmt, ModuleLike as _};
use std::{borrow::Cow, collections::HashMap, marker::PhantomData};

pub type PicusModuleRef = picus::ModuleRef<VarKey>;
pub(super) type PicusExpr = picus::expr::Expr;

#[derive(Clone)]
pub struct PicusModuleLowering<L> {
    module: PicusModuleRef,
    lift_fixed: bool,
    naming_convention: NamingConvention,
    regions: HashMap<RegionIndex, RegionStart>,
    _field: PhantomData<L>,
}

impl<L> PicusModuleLowering<L> {
    pub fn new(
        module: PicusModuleRef,
        lift_fixed: bool,
        regions: HashMap<RegionIndex, RegionStart>,
        naming_convention: NamingConvention,
    ) -> Self {
        Self {
            module,
            lift_fixed,
            regions,
            naming_convention,
            _field: Default::default(),
        }
    }

    pub fn find_region(&self, idx: &RegionIndex) -> Option<RegionStart> {
        self.regions.get(idx).copied()
    }
}

impl<L: LiftLike> PicusModuleLowering<L> {
    fn lower_resolved_query(
        &self,
        query: ResolvedQuery<L>,
        fqn: Option<Cow<FQN>>,
    ) -> Result<PicusExpr> {
        Ok(match query {
            ResolvedQuery::Lit(f) => self.lower(&f, true)?,
            ResolvedQuery::IO(func_io) => {
                let seed = VarKeySeed::named_io(func_io, fqn, self.naming_convention);
                expr::var(&self.module, seed)
            }
        })
    }
}

impl<L: LiftLike> LiftLowering for PicusModuleLowering<L> {
    type F = L::Inner;

    type Output = PicusExpr;

    fn lower_constant(&self, f: &Self::F) -> Result<Self::Output> {
        Ok(expr::r#const(FeltWrap(*f)))
    }

    fn lower_lifted(&self, id: usize, f: Option<&Self::F>) -> Result<Self::Output> {
        if self.lift_fixed {
            Ok(expr::var(
                &self.module,
                VarKeySeed::lifted(id, self.naming_convention),
            ))
        } else if let Some(f) = f {
            Ok(expr::r#const(FeltWrap(*f)))
        } else {
            bail!(
                "Lifted value did not have an inner value and the lowerer was not configured to lift"
            )
        }
    }

    fn lower_add(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output> {
        Ok(expr::add(lhs, rhs))
    }

    fn lower_sub(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output> {
        Ok(expr::sub(lhs, rhs))
    }

    fn lower_mul(&self, lhs: &Self::Output, rhs: &Self::Output) -> Result<Self::Output> {
        Ok(expr::mul(lhs, rhs))
    }

    fn lower_neg(&self, expr: &Self::Output) -> Result<Self::Output> {
        Ok(expr::neg(expr))
    }

    fn lower_double(&self, expr: &Self::Output) -> Result<Self::Output> {
        Ok(expr::add(expr, expr))
    }

    fn lower_square(&self, expr: &Self::Output) -> Result<Self::Output> {
        Ok(expr::mul(expr, expr))
    }

    fn lower_invert(&self, _expr: &Self::Output) -> Result<Self::Output> {
        bail!("Inversion operation is not expressible in Picus")
    }

    fn lower_sqrt_ratio(&self, _lhs: &Self::Output, _rhs: &Self::Output) -> Result<Self::Output> {
        todo!()
    }

    fn lower_cond_select(
        &self,
        _cond: bool,
        _then: &Self::Output,
        _else: &Self::Output,
    ) -> Result<Self::Output> {
        bail!("Conditional select operation is not expressible in Picus")
    }
}

impl<L: LiftLike> Lowering for PicusModuleLowering<L> {
    type CellOutput = PicusExpr;

    type F = L;

    fn generate_constraint(
        &self,
        op: BinaryBoolOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()> {
        self.module.borrow_mut().add_constraint(match op {
            BinaryBoolOp::Eq => expr::eq(lhs, rhs),
            BinaryBoolOp::Lt => expr::lt(lhs, rhs),
            BinaryBoolOp::Le => expr::le(lhs, rhs),
            BinaryBoolOp::Gt => expr::gt(lhs, rhs),
            BinaryBoolOp::Ge => expr::ge(lhs, rhs),
            BinaryBoolOp::Ne => unimplemented!(),
        });
        Ok(())
    }

    fn num_constraints(&self) -> usize {
        self.module.constraints_len()
    }

    fn generate_comment(&self, s: String) -> Result<()> {
        self.module.borrow_mut().add_stmt(stmt::comment(s));
        Ok(())
    }

    fn generate_call(
        &self,
        name: &str,
        selectors: &[Self::CellOutput],
        queries: &[Self::CellOutput],
    ) -> Result<()> {
        self.module.borrow_mut().add_stmt(stmt::call(
            name.to_owned(),
            selectors
                .iter()
                .chain(queries.iter())
                .map(Clone::clone)
                .collect(),
            0,
            &self.module,
            self.naming_convention,
        ));
        Ok(())
    }

    fn lower_sum(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        Ok(expr::add(lhs, rhs))
    }

    fn lower_product(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        Ok(expr::mul(lhs, rhs))
    }

    fn lower_neg(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::neg(expr))
    }

    fn lower_scaled(
        &self,
        expr: &Self::CellOutput,
        scale: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        Ok(expr::mul(expr, scale))
    }

    fn lower_challenge(&self, _challenge: &Challenge) -> Result<Self::CellOutput> {
        unimplemented!()
    }

    fn lower_selector(
        &self,
        sel: &Selector,
        resolver: &dyn SelectorResolver,
    ) -> Result<Self::CellOutput> {
        match resolver.resolve_selector(sel)? {
            ResolvedSelector::Const(value) => Lowering::lower_constant(self, value.to_f()),
            ResolvedSelector::Arg(arg_no) => Ok(expr::var(
                &self.module,
                VarKeySeed::io(arg_no, self.naming_convention),
            )),
        }
    }

    fn lower_advice_query(
        &self,
        query: &AdviceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput> {
        let (res, fqn) = resolver.resolve_advice_query(query)?;
        self.lower_resolved_query(res, fqn)
    }

    fn lower_instance_query(
        &self,
        query: &InstanceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput> {
        self.lower_resolved_query(resolver.resolve_instance_query(query)?, None)
    }

    fn lower_fixed_query(
        &self,
        query: &FixedQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput> {
        self.lower_resolved_query(resolver.resolve_fixed_query(query)?, None)
    }

    fn lower_constant(&self, f: Self::F) -> Result<Self::CellOutput> {
        let expr = self.lower(&f, true)?;
        log::debug!(
            "[PicusBackend::lower_constant] Constant value {f:?} becomes expression {expr:?}"
        );
        Ok(expr)
    }
}
