use picus::vars::VarStr;

use super::vars::VarKey;

pub fn mk_io<F, I, O, C>(count: usize, f: F, c: C) -> impl Iterator<Item = O>
where
    O: Into<VarKey> + Into<VarStr>,
    I: From<usize>,
    F: Fn(I, C) -> O + 'static,
    C: Copy,
{
    (0..count).map(move |i| f(i.into(), c))
}
