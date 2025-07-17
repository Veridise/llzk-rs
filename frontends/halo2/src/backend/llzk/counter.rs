use std::{cell::RefCell, ops::RangeFrom};

pub struct Counter {
    inner: RefCell<RangeFrom<usize>>,
}

impl Counter {
    pub fn next(&self) -> usize {
        self.inner.borrow_mut().next().unwrap()
    }
}
