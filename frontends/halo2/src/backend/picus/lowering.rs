use super::{
    vars::{NamingConvention, VarKey, VarKeySeed},
    FeltWrap,
};
#[cfg(feature = "lift-field-operations")]
use crate::ir::lift::{LiftLike, LiftLowering};
use crate::{
    backend::{
        codegen::queue::RegionStartResolver,
        func::{ArgNo, FieldId, FuncIO},
        lowering::{tag::LoweringOutput, Lowering},
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    halo2::{
        AdviceQuery, Challenge, FixedQuery, InstanceQuery, RegionIndex, RegionStart, Selector,
    },
    ir::CmpOp,
    synthesis::regions::{RegionIndexToStart, FQN},
    LoweringField,
};
use anyhow::Result;
use picus::{expr, stmt, ModuleLike as _};
use std::{borrow::Cow, collections::HashMap, marker::PhantomData};

pub type PicusModuleRef = picus::ModuleRef<VarKey>;
pub(super) type PicusExpr = picus::expr::Expr;

#[derive(Clone)]
pub struct PicusModuleLowering<L> {
    module: PicusModuleRef,
    #[cfg(feature = "lift-field-operations")]
    lift_fixed: bool,
    naming_convention: NamingConvention,
    regions: Option<RegionIndexToStart>,
    _field: PhantomData<L>,
}

impl<L> PicusModuleLowering<L> {
    pub fn new(
        module: PicusModuleRef,
        #[cfg(feature = "lift-field-operations")] lift_fixed: bool,
        regions: Option<HashMap<RegionIndex, RegionStart>>,
        naming_convention: NamingConvention,
    ) -> Self {
        Self {
            module,
            #[cfg(feature = "lift-field-operations")]
            lift_fixed,
            regions,
            naming_convention,
            _field: Default::default(),
        }
    }
}

impl<L: LoweringField> PicusModuleLowering<L> {
    pub fn lower_func_io(&self, func_io: FuncIO, fqn: Option<Cow<FQN>>) -> PicusExpr {
        let seed = VarKeySeed::named_io(func_io, fqn, self.naming_convention);
        expr::var(&self.module, seed)
    }

    fn lower_resolved_query(
        &self,
        query: ResolvedQuery<L>,
        fqn: Option<Cow<FQN>>,
    ) -> Result<PicusExpr> {
        Ok(match query {
            ResolvedQuery::Lit(f) => Lowering::lower_constant(self, f)?,
            ResolvedQuery::IO(func_io) => self.lower_func_io(func_io, fqn),
        })
    }
}

impl<L> RegionStartResolver for PicusModuleLowering<L> {
    fn find(&self, idx: RegionIndex) -> Result<RegionStart> {
        self.regions
            .as_ref()
            .and_then(|regions| regions.get(&idx).copied())
            .ok_or_else(|| anyhow::anyhow!("Failed to get start row for region {}", *idx))
    }
}

#[cfg(feature = "lift-field-operations")]
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
            anyhow::bail!(
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
        unimplemented!()
    }

    fn lower_sqrt_ratio(&self, _lhs: &Self::Output, _rhs: &Self::Output) -> Result<Self::Output> {
        unimplemented!()
    }

    fn lower_cond_select(
        &self,
        _cond: bool,
        _then: &Self::Output,
        _else: &Self::Output,
    ) -> Result<Self::Output> {
        unimplemented!()
    }
}

impl LoweringOutput for PicusExpr {}

impl<L: LoweringField> Lowering for PicusModuleLowering<L> {
    type CellOutput = PicusExpr;

    type F = L;

    fn generate_constraint(
        &self,
        op: CmpOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()> {
        self.module.borrow_mut().add_constraint(match op {
            CmpOp::Eq => expr::eq(lhs, rhs),
            CmpOp::Lt => expr::lt(lhs, rhs),
            CmpOp::Le => expr::le(lhs, rhs),
            CmpOp::Gt => expr::gt(lhs, rhs),
            CmpOp::Ge => expr::ge(lhs, rhs),
            CmpOp::Ne => unimplemented!(),
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
        inputs: &[Self::CellOutput],
        outputs: &[FuncIO],
    ) -> Result<()> {
        let stmt = stmt::call(
            name.to_owned(),
            inputs.to_vec(),
            outputs
                .iter()
                .copied()
                .map(|o| self.lower_func_io(o, None))
                .collect(),
        )?;
        self.module.borrow_mut().add_stmt(stmt);
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
            ResolvedSelector::Arg(arg_no) => Ok(self.lower_func_io(arg_no.into(), None)),
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

    #[cfg(feature = "lift-field-operations")]
    fn lower_constant(&self, f: Self::F) -> Result<Self::CellOutput> {
        let expr = self.lower(&f, true)?;
        log::debug!(
            "[PicusBackend::lower_constant] Constant value {f:?} becomes expression {expr:?}"
        );
        Ok(expr)
    }

    #[cfg(not(feature = "lift-field-operations"))]
    fn lower_constant(&self, f: Self::F) -> Result<Self::CellOutput> {
        let expr = expr::r#const(FeltWrap::from(f));
        log::debug!(
            "[PicusBackend::lower_constant] Constant value {f:?} becomes expression {expr:?}"
        );
        Ok(expr)
    }

    fn generate_assume_deterministic(&self, func_io: FuncIO) -> Result<()> {
        let stmt = stmt::assume_deterministic(self.lower_func_io(func_io, None))?;
        self.module.borrow_mut().add_stmt(stmt);
        Ok(())
    }

    fn lower_eq(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::eq(lhs, rhs))
    }

    fn lower_and(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput> {
        Ok(expr::and(lhs, rhs))
    }

    fn lower_or(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::or(lhs, rhs))
    }

    fn generate_assert(&self, expr: &Self::CellOutput) -> Result<()> {
        let stmt = stmt::constrain(expr.clone());
        self.module.borrow_mut().add_stmt(stmt);
        Ok(())
    }

    fn lower_function_input(&self, i: usize) -> FuncIO {
        ArgNo::from(i).into()
    }

    fn lower_function_output(&self, o: usize) -> FuncIO {
        FieldId::from(o).into()
    }

    fn lower_funcio<IO>(&self, io: IO) -> Result<Self::CellOutput>
    where
        IO: Into<FuncIO>,
    {
        Ok(self.lower_func_io(io.into(), None))
    }

    fn lower_lt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::lt(lhs, rhs))
    }

    fn lower_le(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::le(lhs, rhs))
    }

    fn lower_gt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::gt(lhs, rhs))
    }

    fn lower_ge(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::ge(lhs, rhs))
    }

    fn lower_ne(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::ne(lhs, rhs))
    }

    fn lower_not(&self, value: &Self::CellOutput) -> Result<Self::CellOutput> {
        Ok(expr::not(value))
    }
}
