use crate::halo2::{Column, ColumnType};
use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

/// A matrix of columns and rows of a certain type (Instance, Fixed, Advice, etc)
#[derive(Debug)]
pub struct Matrix<T, V: Default> {
    data: HashMap<(usize, usize), V>,
    default: V,
    _marker: PhantomData<T>,
}

impl<T, V: Default> Default for Matrix<T, V> {
    fn default() -> Self {
        Self {
            data: Default::default(),
            default: Default::default(),
            _marker: Default::default(),
        }
    }
}

impl<T: ColumnType, V: Default> Index<(Column<T>, usize)> for Matrix<T, V> {
    type Output = V;

    fn index(&self, index: (Column<T>, usize)) -> &Self::Output {
        self.data
            .get(&(index.0.index(), index.1))
            .unwrap_or(&self.default)
    }
}

impl<T: ColumnType, V: Default> IndexMut<(Column<T>, usize)> for Matrix<T, V> {
    fn index_mut(&mut self, index: (Column<T>, usize)) -> &mut Self::Output {
        let key = (index.0.index(), index.1);
        //if !self.data.contains_key(&key) {
        //    self.data.insert(key, V::default());
        //}
        self.data.entry(key).or_insert_with(|| V::default());
        self.data.get_mut(&key).unwrap()
    }
}
