use crate::{
    error::to_plonk_error,
    halo2::{
        Advice, Any, Assigned, Cell, Challenge, Column, Error, Field, Fixed, Instance, Layouter,
        Region, RegionIndex, RegionLayouter, Selector, Table, Value,
    },
    io::CircuitIO,
};

#[derive(Debug)]
pub struct RegionLayouterHelper<'r, F: Field> {
    inner: Region<'r, F>,
    detected_region_index: Option<RegionIndex>,
}

impl<'r, F: Field> RegionLayouterHelper<'r, F> {
    pub fn wrapped<'w>(&'w mut self) -> Region<'w, F>
    where
        'r: 'w,
    {
        Region::from(self as &'w mut dyn RegionLayouter<F>)
    }

    pub fn index(&self) -> Option<RegionIndex> {
        self.detected_region_index
    }

    fn detect_index<P>(&mut self, cell: Cell, payload: P) -> Result<P, Error> {
        match self.detected_region_index {
            Some(known_index) => {
                if known_index != cell.region_index {
                    return Err(to_plonk_error(format!(
                        "Incosistent region index in cells. Known index is {} and the given index is {}",
                        *known_index, *cell.region_index
                    )));
                }
            }
            None => {
                self.detected_region_index = Some(cell.region_index);
            }
        }
        Ok(payload)
    }

    fn detect_index_cell_payload(&mut self, cell: Cell) -> Result<Cell, Error> {
        self.detect_index(cell, cell)
    }
}

impl<'r, F: Field> From<Region<'r, F>> for RegionLayouterHelper<'r, F> {
    fn from(inner: Region<'r, F>) -> Self {
        Self {
            inner,
            detected_region_index: None,
        }
    }
}

impl<F: Field> RegionLayouter<F> for RegionLayouterHelper<'_, F> {
    fn enable_selector<'v>(
        &'v mut self,
        annotation: &'v (dyn Fn() -> String + 'v),
        selector: &Selector,
        offset: usize,
    ) -> Result<(), Error> {
        selector.enable(&mut self.inner, offset)
    }

    fn name_column<'v>(
        &'v mut self,
        annotation: &'v (dyn Fn() -> String + 'v),
        column: Column<Any>,
    ) {
        self.inner.name_column(annotation, column)
    }

    fn assign_advice<'v>(
        &'v mut self,
        annotation: &'v (dyn Fn() -> String + 'v),
        column: Column<Advice>,
        offset: usize,
        to: &'v mut (dyn FnMut() -> Value<Assigned<F>> + 'v),
    ) -> Result<Cell, Error> {
        self.inner
            .assign_advice(annotation, column, offset, to)
            .and_then(|cell| self.detect_index_cell_payload(cell.cell()))
    }

    fn assign_advice_from_constant<'v>(
        &'v mut self,
        annotation: &'v (dyn Fn() -> String + 'v),
        column: Column<Advice>,
        offset: usize,
        constant: Assigned<F>,
    ) -> Result<Cell, Error> {
        self.inner
            .assign_advice_from_constant(annotation, column, offset, constant)
            .and_then(|cell| self.detect_index_cell_payload(cell.cell()))
    }

    fn assign_advice_from_instance<'v>(
        &mut self,
        annotation: &'v (dyn Fn() -> String + 'v),
        instance: Column<Instance>,
        row: usize,
        advice: Column<Advice>,
        offset: usize,
    ) -> Result<(Cell, Value<F>), Error> {
        self.inner
            .assign_advice_from_instance(annotation, instance, row, advice, offset)
            .and_then(|assigned| {
                let cell = assigned.cell();
                let value = assigned.value().copied();
                self.detect_index(cell, (cell, value))
            })
    }

    fn instance_value(
        &mut self,
        instance: Column<Instance>,
        row: usize,
    ) -> Result<Value<F>, Error> {
        self.inner.instance_value(instance, row)
    }

    fn assign_fixed<'v>(
        &'v mut self,
        annotation: &'v (dyn Fn() -> String + 'v),
        column: Column<Fixed>,
        offset: usize,
        to: &'v mut (dyn FnMut() -> Value<Assigned<F>> + 'v),
    ) -> Result<Cell, Error> {
        self.inner
            .assign_fixed(annotation, column, offset, to)
            .and_then(|cell| self.detect_index_cell_payload(cell.cell()))
    }

    fn constrain_constant(&mut self, cell: Cell, constant: Assigned<F>) -> Result<(), Error> {
        self.inner.constrain_constant(cell, constant)
    }

    fn constrain_equal(&mut self, left: Cell, right: Cell) -> Result<(), Error> {
        self.inner.constrain_equal(left, right)
    }
}
