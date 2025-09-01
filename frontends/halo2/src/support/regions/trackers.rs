use std::{collections::VecDeque, ops::RangeInclusive};

use crate::halo2::RegionIndex;

/// Tracks the region's indices that the regions layouter has seen.
pub trait RegionIndexTracker {
    fn update(&mut self, index: RegionIndex);

    fn indices(self) -> TrackerIter;

    fn contains(&self, index: RegionIndex) -> bool;
}

pub struct TrackerIter(Box<dyn Iterator<Item = RegionIndex>>);

impl Iterator for TrackerIter {
    type Item = RegionIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.as_mut().next()
    }
}

/// Keeps track of the region indices by maintaining the range of the earliest and latest regions
/// seen. Regions defined in the middle are considered part of the isolated set even if they
/// weren't directly assigned with the layouter.
#[derive(Debug, Default)]
pub struct RangeTracker {
    regions: Option<RangeInclusive<usize>>,
}

impl RegionIndexTracker for RangeTracker {
    fn update(&mut self, index: RegionIndex) {
        let index = *index;
        let range = self.regions.get_or_insert(index..=index);
        let start = range.start().min(&index);
        let end = range.end().max(&index);
        *range = *start..=*end;
    }

    fn indices(self) -> TrackerIter {
        self.into_iter()
    }

    fn contains(&self, index: RegionIndex) -> bool {
        self.regions
            .as_ref()
            .map(|r| r.contains(&*index))
            .unwrap_or_default()
    }
}

impl IntoIterator for RangeTracker {
    type Item = RegionIndex;

    type IntoIter = TrackerIter;

    fn into_iter(self) -> Self::IntoIter {
        TrackerIter(Box::new(
            self.regions.unwrap_or_else(|| 1..=0).map(RegionIndex::from),
        ))
    }
}

/// Keeps track of only the region indices that the layouter directly assigned.
#[derive(Debug, Default)]
pub struct SetTracker {
    regions: VecDeque<usize>,
}

impl RegionIndexTracker for SetTracker {
    fn update(&mut self, index: RegionIndex) {
        if let Err(pos) = self.regions.binary_search(&index) {
            self.regions.insert(pos, *index);
        }
    }
    fn indices(self) -> TrackerIter {
        self.into_iter()
    }

    fn contains(&self, index: RegionIndex) -> bool {
        self.regions.contains(&*index)
    }
}

impl IntoIterator for SetTracker {
    type Item = RegionIndex;

    type IntoIter = TrackerIter;

    fn into_iter(self) -> Self::IntoIter {
        TrackerIter(Box::new(self.regions.into_iter().map(RegionIndex::from)))
    }
}

#[cfg(test)]
mod tests {

    use crate::halo2::RegionIndex;

    use super::*;

    fn range(r: RangeInclusive<usize>) -> Vec<RegionIndex> {
        r.map(RegionIndex::from).collect()
    }

    fn common_test(
        indices: impl IntoIterator<Item = usize>,
        expected: Vec<RegionIndex>,
        mut tracker: impl RegionIndexTracker,
    ) {
        for i in indices {
            tracker.update(i.into());
        }
        let indices = tracker.indices().into_iter().collect::<Vec<_>>();
        assert_eq!(indices, expected);
    }

    /// Test for the RangeTracker tracker.
    fn range_test(indices: impl IntoIterator<Item = usize>, target: RangeInclusive<usize>) {
        common_test(indices, range(target), RangeTracker::default());
    }

    /// Test for the SetTracker tracker.
    fn set_test(indices: impl IntoIterator<Item = usize>, target: impl IntoIterator<Item = usize>) {
        common_test(
            indices,
            target.into_iter().map(RegionIndex::from).collect(),
            SetTracker::default(),
        );
    }

    #[test]
    fn update_range_extent() {
        range_test([1], 1..=1);
        range_test([1, 1], 1..=1);
        range_test([1, 1, 1], 1..=1);
        range_test([1, 2], 1..=2);
        range_test([1, 1, 2], 1..=2);
        range_test([1, 2, 2], 1..=2);
        range_test([2, 1, 1], 1..=2);
        range_test([1, 2, 3], 1..=3);
        range_test([1, 3], 1..=3);
        range_test([2, 1, 3], 1..=3);
    }

    #[test]
    fn update_set_extent() {
        set_test([1], [1]);
        set_test([1, 1], [1]);
        set_test([1, 1, 1], [1]);
        set_test([1, 2], [1, 2]);
        set_test([1, 1, 2], [1, 2]);
        set_test([1, 2, 2], [1, 2]);
        set_test([2, 1, 1], [1, 2]);
        set_test([1, 2, 3], [1, 2, 3]);
        set_test([1, 3], [1, 3]);
        set_test([2, 1, 3], [1, 2, 3]);
    }
}
