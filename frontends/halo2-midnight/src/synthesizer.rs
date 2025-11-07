//! Types for supporting circuit synthesis.

use std::cell::RefCell;
use std::marker::PhantomData;

use ff::Field;
use halo2_frontend_core::synthesis::SynthesizerLike;
use midnight_proofs::plonk::{Advice, Challenge, FloorPlanner};
use midnight_proofs::{
    circuit::{
        Value,
        groups::{GroupKey, GroupKeyInstance, RegionsGroup},
    },
    plonk::{Any, Assignment, Circuit, Column, Error, Fixed, Instance, Selector},
    utils::rational::Rational,
};

use crate::circuit::groups::_RegionsGroup;
use crate::plonk::{_Advice, _Any, _Column, _Fixed, _Selector, ConstraintSystem};

/// Implementation of [`Assignment`] compatible with [`Synthesizer`].
#[derive(Debug)]
pub struct SynthesizerAssignment<'a, F: Field, S: SynthesizerLike<F>> {
    synthetizer: &'a mut S,
    _marker: PhantomData<F>,
}

impl<'a, F: Field, S: SynthesizerLike<F>> SynthesizerAssignment<'a, F, S> {
    /// Synthesizes the given circuit, recording the synthesis information in the synthetizer.
    pub fn synthesize<C: Circuit<F>>(
        circuit: &C,
        config: C::Config,
        synthetizer: &'a mut S,
        cs: &ConstraintSystem<F>,
    ) -> Result<(), Error> {
        let mut assign = Self {
            synthetizer,
            _marker: Default::default(),
        };
        let constants = cs.inner().constants().clone();
        C::FloorPlanner::synthesize(&mut assign, circuit, config, constants)?;

        Ok(())
    }
}

impl<F: Field, S: SynthesizerLike<F>> Assignment<F> for SynthesizerAssignment<'_, F, S> {
    fn enter_region<NR, N>(&mut self, region_name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.synthetizer.enter_region(region_name().into());
    }

    fn exit_region(&mut self) {
        self.synthetizer.exit_region();
    }

    fn enable_selector<A, AR>(&mut self, _: A, selector: &Selector, row: usize) -> Result<(), Error>
    where
        AR: Into<String>,
        A: FnOnce() -> AR,
    {
        self.synthetizer
            .enable_selector(&_Selector::from(*selector), row);
        Ok(())
    }

    fn query_instance(&self, _column: Column<Instance>, _row: usize) -> Result<Value<F>, Error> {
        Ok(Value::unknown())
    }

    fn assign_advice<V, VR, A, AR>(
        &mut self,
        _name: A,
        advice: Column<Advice>,
        row: usize,
        _value: V,
    ) -> Result<(), Error>
    where
        VR: Into<Rational<F>>,
        AR: Into<String>,
        V: FnOnce() -> Value<VR>,
        A: FnOnce() -> AR,
    {
        self.synthetizer
            .on_advice_assigned(_Column::<_Advice>::from(advice), row);
        Ok(())
    }

    fn assign_fixed<V, VR, A, AR>(
        &mut self,
        _: A,
        fixed: Column<Fixed>,
        row: usize,
        value: V,
    ) -> Result<(), Error>
    where
        VR: Into<Rational<F>>,
        AR: Into<String>,
        V: FnOnce() -> Value<VR>,
        A: FnOnce() -> AR,
    {
        let value = value().map(|f| f.into().evaluate());
        self.synthetizer.on_fixed_assigned(
            _Column::<_Fixed>::from(fixed),
            row,
            steal(&value).ok_or_else(|| {
                to_plonk_error(format!(
                    "Unknown value in fixed cell ({}, {row})",
                    fixed.index()
                ))
            })?,
        );
        Ok(())
    }

    fn copy(
        &mut self,
        from: Column<Any>,
        from_row: usize,
        to: Column<Any>,
        to_row: usize,
    ) -> Result<(), Error> {
        self.synthetizer.copy(
            _Column::<_Any>::from(from),
            from_row,
            _Column::<_Any>::from(to),
            to_row,
        );
        Ok(())
    }

    fn fill_from_row(
        &mut self,
        column: Column<Fixed>,
        row: usize,
        value: Value<Rational<F>>,
    ) -> Result<(), Error> {
        self.synthetizer.fill_table(
            _Column::<_Fixed>::from(column),
            row,
            steal(&value.map(|f| f.evaluate())).ok_or_else(|| {
                to_plonk_error(format!(
                    "Unknown value in fixed cell ({}, {row})",
                    column.index()
                ))
            })?,
        );
        Ok(())
    }

    fn push_namespace<NR, N>(&mut self, name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        self.synthetizer.push_namespace(name().into());
    }

    fn pop_namespace(&mut self, name: Option<String>) {
        self.synthetizer.pop_namespace(name);
    }

    fn annotate_column<A, AR>(&mut self, _: A, _: Column<Any>)
    where
        AR: Into<String>,
        A: FnOnce() -> AR,
    {
        // Do nothing.
    }

    fn get_challenge(&self, _: Challenge) -> Value<F> {
        Value::unknown()
    }

    fn enter_group<NR, N, K>(&mut self, name: N, key: K)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
        K: GroupKey,
    {
        self.synthetizer
            .enter_group(name().into(), *GroupKeyInstance::from(key));
    }

    fn exit_group(&mut self, meta: RegionsGroup) {
        self.synthetizer.exit_group(_RegionsGroup::from(meta))
    }
}

struct ValueStealer<T> {
    data: RefCell<Option<T>>,
}

impl<T: Clone> ValueStealer<T> {
    fn new() -> Self {
        Self {
            data: RefCell::new(None),
        }
    }

    fn steal(&self, value: Value<T>) -> Option<T> {
        value.map(|t| self.data.replace(Some(t)));
        self.data.replace(None)
    }
}

/// Transforms a [`Value`] into an [`Option`], returning None if the value is unknown.
pub fn steal<T: Clone>(value: &Value<T>) -> Option<T> {
    let stealer = ValueStealer::<T>::new();
    stealer.steal(value.clone())
}

fn to_plonk_error<E>(error: E) -> Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    Error::Transcript(std::io::Error::other(error))
}
