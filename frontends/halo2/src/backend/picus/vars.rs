use anyhow::Result;

#[derive(Clone, Copy)]
pub enum VarKind {
    Input,
    Output,
    Temporary,
}

pub trait VarAllocator<'a> {
    type Meta;

    fn allocate<M: Into<Self::Meta>>(&'a self, kind: &VarKind, meta: M) -> Result<&'a str>;
}
