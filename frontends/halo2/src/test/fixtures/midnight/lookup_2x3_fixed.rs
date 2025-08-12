use group::ff::Field;
use midnight_halo2_proofs::circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value};
use midnight_halo2_proofs::plonk::{
    Advice, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector, TableColumn,
};
use midnight_halo2_proofs::poly::Rotation;
use std::iter;
use std::marker::PhantomData;

use crate::lookups::callbacks::LookupCallbacks;
use crate::{CircuitCallbacks, CircuitIO};

#[derive(Debug, Clone)]
pub struct Lookup2x3Config {
    #[allow(dead_code)]
    pub col_fixed: Column<Fixed>,
    pub lookup_column: [TableColumn; 2],
    pub col_f: Column<Advice>,
    pub col_a: Column<Advice>,
    pub col_b: Column<Advice>,
    pub col_c: Column<Advice>,
    pub selector: Selector,
    pub instance: Column<Instance>,
}

#[derive(Debug, Clone)]
struct Lookup2x3Chip<F: Field> {
    config: Lookup2x3Config,
    _marker: PhantomData<F>,
}

impl<F: Field> Lookup2x3Chip<F> {
    pub fn construct(config: Lookup2x3Config) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> Lookup2x3Config {
        let col_fixed = meta.fixed_column();
        let col_a = meta.advice_column();
        let col_f = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let selector = meta.complex_selector();
        let instance = meta.instance_column();

        meta.enable_constant(col_fixed);
        meta.enable_equality(col_a);
        meta.enable_equality(col_f);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        let lookup_column = [meta.lookup_table_column(), meta.lookup_table_column()];

        meta.lookup("lookup test", |meta| {
            let s = meta.query_selector(selector);
            let f = meta.query_fixed(col_fixed, Rotation::cur());
            let a = meta.query_advice(col_a, Rotation::cur());

            vec![(s.clone() * f, lookup_column[0]), (s * a, lookup_column[1])]
        });

        Lookup2x3Config {
            col_fixed,
            lookup_column,
            col_a,
            col_f,
            col_b,
            col_c,
            selector,
            instance,
        }
    }

    // Utility function for creating a field element from a native value. Complexity is O(n) where
    // n is the value of the number so don't use very large numbers with this.
    fn f(&self, n: usize) -> Value<F> {
        Value::known(iter::repeat(F::ONE).take(n).sum())
    }

    #[allow(clippy::type_complexity)]
    pub fn assign_table(&self, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "table",
            |mut table| {
                let fst = [10, 20, 30];
                let snd = [7, 11, 13];

                fst.into_iter()
                    .zip(snd.into_iter())
                    .enumerate()
                    .flat_map(|(offset, (x, y))| [(offset, x), (offset, y)])
                    .map(|(offset, n)| (offset, self.f(n)))
                    .zip(self.config.lookup_column.iter().cycle())
                    .try_for_each(|((offset, v), t)| {
                        table.assign_cell(
                            || format!("lookup col {}", t.inner().index()),
                            *t,
                            offset,
                            || -> Value<F> { v },
                        )
                    })
            },
        )
    }

    #[allow(clippy::type_complexity)]
    pub fn assign_first_row(
        &self,
        mut layouter: impl Layouter<F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "first row",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;

                let fixed_cell = region.assign_fixed(
                    || "-1",
                    self.config.col_fixed,
                    0,
                    || -> Value<F> { Value::known(-F::ONE) },
                )?;

                let a_cell = region.assign_advice_from_instance(
                    || "a",
                    self.config.instance,
                    0,
                    self.config.col_a,
                    0,
                )?;

                let b_cell = region.assign_advice(
                    || "-1 * a",
                    self.config.col_b,
                    0,
                    || a_cell.value().copied() * fixed_cell.value(),
                )?;

                let c_cell = region.assign_advice(
                    || "a * b",
                    self.config.col_c,
                    0,
                    || a_cell.value().copied() * b_cell.value(),
                )?;

                Ok(c_cell)
            },
        )
    }

    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        cell: &AssignedCell<F, F>,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, row)
    }
}

#[derive(Default)]
pub struct Lookup2x3Circuit<F>(pub PhantomData<F>);

impl<F: Field> Circuit<F> for Lookup2x3Circuit<F> {
    type Config = Lookup2x3Config;
    type FloorPlanner = SimpleFloorPlanner;
    type Params = ();

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        Lookup2x3Chip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = Lookup2x3Chip::construct(config);
        chip.assign_table(layouter.namespace(|| "table"))?;
        let prev_c = chip.assign_first_row(layouter.namespace(|| "first row"))?;

        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 1)?;
        Ok(())
    }
}

impl<F: Field> CircuitCallbacks<F, Self> for Lookup2x3Circuit<F> {
    fn advice_io(_: &<Self as Circuit<F>>::Config) -> CircuitIO<Advice> {
        CircuitIO::empty()
    }

    fn instance_io(config: &<Self as Circuit<F>>::Config) -> CircuitIO<Instance> {
        CircuitIO::new(&[(config.instance, &[0])], &[(config.instance, &[1])])
    }
    fn lookup_callbacks() -> Option<Box<dyn LookupCallbacks<F>>> {
        Some(Box::new(super::LookupCallbackHandler))
    }
}
