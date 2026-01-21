//! General utility functions

use std::cmp::Ordering;

use halo2_frontend_core::table::{Any, Column};

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

pub(crate) fn fmt_columns<'c>(
    columns: impl IntoIterator<Item = &'c Column<Any>>,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    let mut columns = Vec::from_iter(columns);
    columns.sort_by(|a, b| {
        match (a.column_type(), b.column_type()) {
            (Any::Instance, Any::Advice | Any::Fixed) | (Any::Advice, Any::Fixed) => {
                return Ordering::Less;
            }
            (Any::Fixed, Any::Instance | Any::Advice) | (Any::Advice, Any::Instance) => {
                return Ordering::Greater;
            }
            _ => {}
        }
        a.index().cmp(&b.index())
    });
    let columns = columns
        .into_iter()
        .map(|c| {
            format!(
                "{}:{}",
                match c.column_type() {
                    Any::Fixed => "Fix",
                    Any::Advice => "Adv",
                    Any::Instance => "Ins",
                },
                c.index()
            )
        })
        .collect::<Vec<_>>();

    write!(f, "{}", columns.join(", "))
}
