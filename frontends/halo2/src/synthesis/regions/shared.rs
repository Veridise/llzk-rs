use super::FQN;
use std::{collections::HashMap, ops::AddAssign};

/// Data shared across regions.
#[derive(Debug, Default)]
pub struct SharedRegionData {
    advice_names: HashMap<(usize, usize), FQN>,
}

impl SharedRegionData {
    pub fn advice_names(&self) -> &HashMap<(usize, usize), FQN> {
        &self.advice_names
    }

    pub fn advice_names_mut(&mut self) -> &mut HashMap<(usize, usize), FQN> {
        &mut self.advice_names
    }
}

impl AddAssign for SharedRegionData {
    fn add_assign(&mut self, rhs: Self) {
        for (k, v) in rhs.advice_names {
            self.advice_names.insert(k, v);
        }
    }
}
