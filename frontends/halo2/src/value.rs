use std::cell::RefCell;

use crate::halo2::Value;

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

/// Transforms a list of [`Value`] into a list of [`Option`], returning None if the value is unknown.
#[allow(dead_code)]
pub fn steal_many<T: Clone>(values: &[Value<T>]) -> Option<Vec<T>> {
    values.iter().map(steal).collect()
}
