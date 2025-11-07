use crate::{
    CircuitIO,
    backend::func::{ArgNo, FieldId, FuncIO},
    io::{AdviceIO, IOCell, InstanceIO},
    resolvers::{
        ChallengeResolver, FixedQueryResolver, QueryResolver, ResolvedQuery, ResolvedSelector,
        SelectorResolver,
    },
};
use anyhow::{Result, bail};
use ff::Field;
use halo2_frontend_core::{
    info_traits::{ChallengeInfo, QueryInfo, SelectorInfo},
    query::{Advice, Fixed, Instance},
    table::{ColumnType, Rotation},
};

/// When resolving a query it is possible that the same cell is in the input and the output set.
/// This enum configures what kind will be chosen in the case of conflicting.
///
/// The default priority is outputs.
#[derive(Debug, Copy, Clone, Default)]
pub enum ResolutionPriority {
    Input,
    #[default]
    Output,
}

#[derive(Copy, Clone)]
pub struct Row<'io, 'fq, F: Field> {
    pub(super) row: usize,
    advice_io: &'io AdviceIO,
    instance_io: &'io InstanceIO,
    pub(super) fqr: &'fq dyn FixedQueryResolver<F>,
    priority: ResolutionPriority,
}

impl<'io, 'fq, F: Field> Row<'io, 'fq, F> {
    pub fn new(
        row: usize,
        advice_io: &'io AdviceIO,
        instance_io: &'io InstanceIO,
        fqr: &'fq dyn FixedQueryResolver<F>,
    ) -> Self {
        Self {
            fqr,
            row,
            advice_io,
            instance_io,
            priority: Default::default(),
        }
    }

    /// Changes the priority to inputs.
    pub fn prioritize_inputs(mut self) -> Self {
        self.priority = ResolutionPriority::Input;
        self
    }

    /// Changes the priority to outputs.
    pub fn prioritize_outputs(mut self) -> Self {
        self.priority = ResolutionPriority::Output;
        self
    }

    pub(super) fn resolve_rotation(&self, rot: Rotation) -> Result<usize> {
        let row: isize = self.row.try_into()?;
        let rot: isize = rot.try_into()?;
        if -rot > row {
            bail!("Row underflow");
        }
        Ok((row + rot).try_into()?)
    }

    fn resolve_as<O: From<usize> + Into<FuncIO>, C: ColumnType>(
        &self,
        io: &[IOCell<C>],
        col: usize,
        rot: usize,
    ) -> Result<Option<FuncIO>> {
        let target_cell = (col, rot);
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

    /// Resolves the query of the given column type.
    ///
    /// If the cell was detected as part of the IO returns [`FuncIO::Arg`] or [`FuncIO::Field`].
    /// Otherwise returns the fallback value if given. Otherwise fails.
    fn resolve<C: ColumnType>(
        &self,
        io: &CircuitIO<C>,
        col: usize,
        rot: usize,
        fallback: Option<FuncIO>,
    ) -> Result<FuncIO> {
        let as_input = self.resolve_as::<ArgNo, C>(io.inputs(), col, rot)?;
        let as_output = self.resolve_as::<FieldId, C>(io.outputs(), col, rot)?;

        Ok(match (as_input, as_output) {
            (None, None) => {
                fallback.ok_or_else(|| anyhow::anyhow!("Failed to resolve {} cell ({}, {}): Fallback value was required but was not present.", std::any::type_name::<C>(), col, rot))?
            }
            (None, Some(r)) => r,
            (Some(r), None) => r,
            // Cell is both input and output so we need to decide based on priority.
(Some(input), Some(output)) => match self.priority {
    ResolutionPriority::Input => input,
    ResolutionPriority::Output => output,
}
            //(Some(_), Some(_)) => bail!("Failed to resolve {} cell ({}, {}): Query is both an input and an output in main function", std::any::type_name::<C>(), col, rot),
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

    /// General method for resolving advice queries.
    ///
    /// Takes the query and a callback that returns a fallback value.
    /// This fallback value is used when the the advice cell was not found in
    /// the CircuitIO.
    ///
    /// The two users of this function, Row and RegionRow, create different kind of
    /// advice cells. Former creates absolute cells and later creates relative cells.
    /// The function takes a callback instead of the fallback value directly is because
    /// the final row is computed here but that data is required for creating the fallback value.
    /// Additionally, the callers may have more information that is required for creating the kind
    /// of advice cell is required.
    ///
    /// Is pub(super) because is used by RegionRow as well.
    pub(super) fn resolve_advice_query_impl(
        &self,
        query: &dyn QueryInfo<Kind = Advice>,
        fallback: impl FnOnce(usize, usize) -> FuncIO,
    ) -> Result<FuncIO> {
        let rot = self.resolve_rotation(query.rotation())?;
        let col = query.column_index();
        log::debug!("row: {}, rot: {rot}", self.row);
        let fb = fallback(col, rot);
        assert!(matches!(fb, FuncIO::Advice(_)));
        let r = self.resolve(self.advice_io, col, rot, Some(fb))?;
        // Advice cells go second so we need to step the value by the number of instance cells
        // that are of the same type (input or output)
        Ok(self.step_advice_io(r))
    }
}

impl<F: Field> QueryResolver<F> for Row<'_, '_, F> {
    fn resolve_fixed_query(&self, query: &dyn QueryInfo<Kind = Fixed>) -> Result<ResolvedQuery<F>> {
        let row = self.resolve_rotation(query.rotation())?;
        self.fqr.resolve_query(query, row).map(ResolvedQuery::Lit)
    }

    fn resolve_advice_query(
        &self,
        query: &dyn QueryInfo<Kind = Advice>,
    ) -> Result<ResolvedQuery<F>> {
        let r = self.resolve_advice_query_impl(query, FuncIO::advice_abs)?;
        Ok(r.into())
    }

    fn resolve_instance_query(
        &self,
        query: &dyn QueryInfo<Kind = Instance>,
    ) -> Result<ResolvedQuery<F>> {
        let rot = self.resolve_rotation(query.rotation())?;
        let col = query.column_index();
        let r = self.resolve(self.instance_io, col, rot, None)?;
        Ok(r.into())
    }
}

impl<F: Field> SelectorResolver for Row<'_, '_, F> {
    fn resolve_selector(&self, _selector: &dyn SelectorInfo) -> Result<ResolvedSelector> {
        unreachable!()
    }
}

impl<F: Field> ChallengeResolver for Row<'_, '_, F> {
    fn resolve_challenge(&self, challenge: &dyn ChallengeInfo) -> Result<FuncIO> {
        Ok(FuncIO::Challenge(
            challenge.index(),
            challenge.phase(),
            ArgNo::from(
                self.advice_io.inputs_count() + self.instance_io.inputs_count() + challenge.index(),
            ),
        ))
    }
}

impl<F: Field> std::fmt::Debug for Row<'_, '_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Row")
            .field("row", &self.row)
            .field("advice_io", &self.advice_io)
            .field("instance_io", &self.instance_io)
            .finish()
    }
}
