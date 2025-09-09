
//
///// Data shared across regions.
//#[derive(Debug, Default, Clone)]
//pub struct SharedRegionData<F: Copy + std::fmt::Debug> {
//    advice_names: HashMap<(usize, usize), FQN>,
//    pub(super) fixed: FixedData<F>,
//}
//
//impl<F: Copy + std::fmt::Debug> SharedRegionData<F> {
//    pub fn advice_names(&self) -> &HashMap<(usize, usize), FQN> {
//        &self.advice_names
//    }
//
//    pub fn advice_names_mut(&mut self) -> &mut HashMap<(usize, usize), FQN> {
//        &mut self.advice_names
//    }
//
//    pub fn resolve_fixed(&self, column: usize, row: usize) -> Option<Value<F>>
//    where
//        F: Field,
//    {
//        self.fixed.resolve_fixed(column, row)
//    }
//}
//
//impl<F: Copy + std::fmt::Debug> AddAssign for SharedRegionData<F> {
//    fn add_assign(&mut self, rhs: Self) {
//        for (k, v) in rhs.advice_names {
//            self.advice_names.insert(k, v);
//        }
//        self.fixed += rhs.fixed;
//    }
//}
