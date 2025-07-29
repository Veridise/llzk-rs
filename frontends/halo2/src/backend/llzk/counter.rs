use std::{cell::RefCell, ops::RangeFrom};

pub struct Counter {
    inner: RefCell<RangeFrom<usize>>,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            inner: RefCell::new(0..),
        }
    }
}

impl Counter {
    pub fn next(&self) -> usize {
        self.inner.borrow_mut().next().unwrap()
    }
}
