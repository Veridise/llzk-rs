use std::{borrow::Cow, cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::{anyhow, Result};

use crate::{
    backend::{
        func::FuncIO,
        lowering::Lowering,
        resolvers::{
            QueryResolver, ResolvedQuery, ResolvedSelector, ResolversProvider, SelectorResolver,
        },
    },
    expressions::ScopedExpression,
    halo2::{
        AdviceQuery, Expression, FixedQuery, InstanceQuery, RegionIndex, RegionStart, Selector,
    },
    ir::stmt::IRStmt,
    synthesis::regions::{RegionIndexToStart, FQN},
};

pub trait RegionStartResolver {
    fn find(&self, idx: RegionIndex) -> Result<RegionStart>;
}

#[inline]
pub fn create_queue_helper<F>() -> Rc<RefCell<CodegenQueueHelper<F>>> {
    Rc::new(RefCell::new(CodegenQueueHelper::new()))
}

#[derive(Default)]
pub struct CodegenQueueHelper<F> {
    enqueued_stmts: HashMap<RegionIndex, Vec<IRStmt<Expression<F>>>>,
}

impl<F> CodegenQueueHelper<F> {
    pub fn new() -> Self {
        Self {
            enqueued_stmts: Default::default(),
        }
    }

    pub fn enqueue_stmts<'s, I>(&'s mut self, region: RegionIndex, stmts: I) -> Result<()>
    where
        I: IntoIterator<Item = IRStmt<Expression<F>>>,
        I::IntoIter: ExactSizeIterator,
    {
        let stmts = stmts.into_iter();
        let n = stmts.len();
        self.enqueued_stmts.entry(region).or_default().extend(stmts);
        log::debug!(
            "Enqueueing {} statements. Currently enqueued: {}",
            n,
            self.enqueued_stmts.len()
        );
        Ok(())
    }

    pub fn dequeue_stmts<'s, L>(&mut self, scope: &'s L) -> Result<()>
    where
        L: Lowering<F = F> + RegionStartResolver,
    {
        dequeue_stmts_impl(scope, &mut self.enqueued_stmts)
    }
}

fn dequeue_stmts_impl<'s, L>(
    scope: &'s L,
    enqueued_stmts: &mut HashMap<RegionIndex, Vec<IRStmt<Expression<L::F>>>>,
) -> Result<()>
where
    L: Lowering + RegionStartResolver,
{
    // Delete the elements waiting in the queue.
    for (region, stmts) in std::mem::take(enqueued_stmts) {
        comment::begin_comment(scope, region)?;

        for stmt in stmts {
            let query_resolver = OnlyAdviceQueriesResolver::new(region, scope)?;
            let selector_resolver = NullSelectorResolver;
            let stmt = stmt.map(&ScopedExpression::make_ctor((
                query_resolver,
                selector_resolver,
            )));
            scope.lower_stmt(stmt)?;
        }

        comment::end_comment(scope, region)?;
    }
    Ok(())
}

#[derive(Copy, Clone)]
struct NullSelectorResolver;

impl SelectorResolver for NullSelectorResolver {
    fn resolve_selector(&self, _: &Selector) -> Result<ResolvedSelector> {
        Err(anyhow!(
            "Selectors are not supported in in-flight statements"
        ))
    }
}

#[derive(Copy)]
struct OnlyAdviceQueriesResolver<'s, L> {
    region: RegionIndex,
    scope: &'s L,
    start: usize,
}

impl<L> Clone for OnlyAdviceQueriesResolver<'_, L> {
    fn clone(&self) -> Self {
        Self {
            region: self.region,
            scope: self.scope,
            start: self.start,
        }
    }
}

impl<'s, L> OnlyAdviceQueriesResolver<'s, L>
where
    L: RegionStartResolver,
{
    pub fn new(region: RegionIndex, scope: &'s L) -> Result<Self> {
        Ok(Self {
            region,
            scope,
            start: *scope.find(region)?,
        })
    }
}

impl<L: Lowering + RegionStartResolver> QueryResolver<L::F> for OnlyAdviceQueriesResolver<'_, L> {
    fn resolve_fixed_query(&self, _: &FixedQuery) -> Result<ResolvedQuery<L::F>> {
        Err(anyhow!(
            "Fixed cells are not supported in in-flight statements"
        ))
    }

    fn resolve_advice_query(
        &self,
        query: &AdviceQuery,
    ) -> Result<(ResolvedQuery<L::F>, Option<Cow<'_, FQN>>)> {
        let offset: usize = query.rotation().0.try_into()?;
        Ok((
            ResolvedQuery::IO(FuncIO::Advice(query.column_index(), self.start + offset)),
            None,
        ))
    }

    fn resolve_instance_query(&self, _: &InstanceQuery) -> Result<ResolvedQuery<L::F>> {
        Err(anyhow!(
            "Instance cells are not supported in in-flight statements"
        ))
    }
}

mod comment {
    use std::marker::PhantomData;

    use anyhow::Result;

    use crate::{
        backend::lowering::{
            lowerable::{Lowerable, LoweringOutput},
            Lowering,
        },
        halo2::{Field, RegionIndex},
        ir::stmt::IRStmt,
    };

    use super::RegionStartResolver;

    struct Dummy<F>(PhantomData<F>);

    impl<F: Field> Lowerable for Dummy<F> {
        type F = F;

        fn lower<L>(self, _: &L) -> Result<impl Into<LoweringOutput<L>>>
        where
            L: Lowering<F = Self::F> + ?Sized,
        {
            unreachable!();
            #[allow(unreachable_code)]
            Ok(())
        }
    }

    macro_rules! comment {
        ($name:ident, $fmt:literal) => {
            pub fn $name<L>(scope: &L, region: RegionIndex) -> Result<()>
            where
                L: Lowering + RegionStartResolver,
            {
                scope.lower_stmt(IRStmt::<Dummy<L::F>>::comment(format!(
                    $fmt,
                    *region,
                    *scope.find(region)?
                )))
            }
        };
    }

    comment!(
        begin_comment,
        "In-flight statements @ Region {} (start row: {})"
    );
    comment!(
        end_comment,
        "End of in-flight statements @ Region {} (start row: {})"
    );
}
