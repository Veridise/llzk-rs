use std::borrow::Cow;

use crate::backend::func::{ArgNo, FieldId, FuncIO};
use crate::backend::resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver};
use crate::{
    gates::AnyQuery,
    halo2::{AdviceQuery, Field, FixedQuery, InstanceQuery, Selector},
    synthesis::regions::FQN,
};
use anyhow::{anyhow, Result};

//pub mod call_gates;
pub mod inline;

#[derive(Copy, Clone)]
enum IO {
    I(usize),
    O(usize),
}

#[derive(Clone)]
pub struct GateScopedResolver<'a> {
    pub selectors: Vec<&'a Selector>,
    pub queries: Vec<AnyQuery>,
    pub outputs: Vec<AnyQuery>,
}

fn resolve<'a, A, B, I, O>(mut it: I, b: &B, err: &'static str) -> Result<O>
where
    A: PartialEq<B> + 'a,
    I: Iterator<Item = (&'a A, IO)>,
    O: From<FuncIO>,
{
    it.find_map(|(a, io)| -> Option<FuncIO> {
        if a == b {
            Some(match io {
                IO::I(idx) => ArgNo::from(idx).into(),
                IO::O(idx) => FieldId::from(idx).into(),
            })
        } else {
            None
        }
    })
    .map(From::from)
    .ok_or(anyhow!(err))
}

impl<'a> GateScopedResolver<'a> {
    fn selectors(&self) -> impl Iterator<Item = (&'a Selector, IO)> {
        self.selectors
            .iter()
            .copied()
            .enumerate()
            .map(|(idx, s)| (s, IO::I(idx)))
    }

    fn io_queries<'q>(&'q self) -> impl Iterator<Item = (&'q AnyQuery, IO)> {
        let input_base = self.selectors.len();
        self.queries
            .iter()
            .enumerate()
            .map(move |(idx, q)| (q, IO::I(idx + input_base)))
            .chain(
                self.outputs
                    .iter()
                    .enumerate()
                    .map(|(idx, q)| (q, IO::O(idx))),
            )
    }
}

impl<F: Field> QueryResolver<F> for GateScopedResolver<'_> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        resolve(self.io_queries(), query, "Query as argument not found")
    }

    fn resolve_advice_query(
        &self,
        query: &AdviceQuery,
    ) -> Result<(ResolvedQuery<F>, Option<Cow<FQN>>)> {
        Ok((
            resolve(self.io_queries(), query, "Query as argument not found")?,
            None,
        ))
    }

    fn resolve_instance_query(&self, query: &InstanceQuery) -> Result<ResolvedQuery<F>> {
        resolve(self.io_queries(), query, "Query as argument not found")
    }
}

impl SelectorResolver for GateScopedResolver<'_> {
    fn resolve_selector(&self, selector: &Selector) -> Result<ResolvedSelector> {
        resolve(self.selectors(), selector, "Selector as argument not found").and_then(
            |io: FuncIO| match io {
                FuncIO::Arg(arg) => Ok(ResolvedSelector::Arg(arg)),
                _ => anyhow::bail!("Cannot get a selector as anything other than an argument"),
            },
        )
    }
}
