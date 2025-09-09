use super::FQN;
use crate::{
    backend::{
        func::{ArgNo, FieldId, FuncIO},
        resolvers::{QueryResolver, ResolvedQuery, ResolvedSelector, SelectorResolver},
    },
    halo2::*,
    io::IOCell,
    CircuitIO,
};
use anyhow::{bail, Result};
use std::{borrow::Cow, marker::PhantomData};

#[derive(Copy, Clone, Debug)]
pub struct Row<'io> {
    pub(super) row: usize,
    advice_io: &'io CircuitIO<Advice>,
    instance_io: &'io CircuitIO<Instance>,
}

impl<'io> Row<'io> {
    pub fn new(
        row: usize,
        advice_io: &'io CircuitIO<Advice>,
        instance_io: &'io CircuitIO<Instance>,
    ) -> Self {
        Self {
            row,
            advice_io,
            instance_io,
        }
    }

    pub(super) fn resolve_rotation(&self, rot: Rotation) -> Result<usize> {
        let row: isize = self.row.try_into()?;
        let rot: isize = rot.0.try_into()?;
        if -rot > row {
            bail!("Row underflow");
        }
        Ok((row + rot).try_into()?)
    }

    fn resolve_as<O: From<usize> + Into<FuncIO>, C: ColumnType>(
        &self,
        io: &[IOCell<C>],
        col: usize,
        rot: Rotation,
    ) -> Result<Option<FuncIO>> {
        let target_cell = (col, self.resolve_rotation(rot)?);
        Ok(io
            .iter()
            .map(|(col, row)| (col.index(), *row))
            .enumerate()
            .find_map(|(idx, cell)| {
                if cell == target_cell {
                    Some(O::from(idx))
                } else {
                    None
                }
            })
            .map(Into::into))
    }

    fn resolve<C: ColumnType>(
        &self,
        io: &CircuitIO<C>,
        col: usize,
        rot: Rotation,
    ) -> Result<FuncIO> {
        let as_input = self.resolve_as::<ArgNo, C>(io.inputs(), col, rot)?;
        let as_output = self.resolve_as::<FieldId, C>(io.outputs(), col, rot)?;

        Ok(match (as_input, as_output) {
            (None, None) => FuncIO::advice_abs(col, self.resolve_rotation(rot)?),
            (None, Some(r)) => r,
            (Some(r), None) => r,
            (Some(_), Some(_)) => bail!("Query is both an input and an output in main function"),
        })
    }

    fn step_advice_io(&self, io: FuncIO) -> FuncIO {
        match io {
            FuncIO::Arg(arg_no) => (arg_no.offset_by(self.instance_io.inputs().len())).into(),
            FuncIO::Field(field_id) => {
                (field_id.offset_by(self.instance_io.outputs().len())).into()
            }
            io => io,
        }
    }
}

impl<F: Field> QueryResolver<F> for Row<'_> {
    fn resolve_fixed_query(&self, query: &FixedQuery) -> Result<ResolvedQuery<F>> {
        let row = self.resolve_rotation(query.rotation())?;
        Ok(ResolvedQuery::IO(FuncIO::fixed_abs(
            query.column_index(),
            row,
        )))
    }

    fn resolve_advice_query<'a>(
        &'a self,
        query: &AdviceQuery,
    ) -> Result<(ResolvedQuery<F>, Option<Cow<'a, FQN>>)> {
        let r = self.resolve(self.advice_io, query.column_index(), query.rotation())?;
        // Advice cells go second so we need to step the value by the number of instance cells
        // that are of the same type (input or output)
        let r = self.step_advice_io(r);
        Ok((r.into(), None))
    }

    fn resolve_instance_query(&self, query: &InstanceQuery) -> Result<ResolvedQuery<F>> {
        let r = self.resolve(self.instance_io, query.column_index(), query.rotation())?;
        Ok(r.into())
    }
}

impl SelectorResolver for Row<'_> {
    fn resolve_selector(&self, _selector: &Selector) -> Result<ResolvedSelector> {
        unreachable!()
    }
}
