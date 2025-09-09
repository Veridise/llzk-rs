//! General utility functions

/// Returns the cartesian product of two iterators.
///
/// Clones the iterator of the right hand side for each element in the left hand side.
#[inline]
pub fn product<'a, L: Clone + 'a, R: 'a>(
    lhs: impl IntoIterator<Item = L> + 'a,
    rhs: impl IntoIterator<Item = R> + Clone + 'a,
) -> impl Iterator<Item = (L, R)> + 'a {
    lhs.into_iter()
        .flat_map(move |lhs| rhs.clone().into_iter().map(move |rhs| (lhs.clone(), rhs)))
}
