use std::fmt;

/// Argument number of a function
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ArgNo(usize);

impl From<usize> for ArgNo {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl ArgNo {
    pub fn offset_by(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }
}

impl fmt::Display for ArgNo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An identifier that Backend::FuncOutput will use to identify an output field in the function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldId(usize);

impl From<usize> for FieldId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl FieldId {
    pub fn offset_by(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }
}

impl fmt::Display for FieldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub enum FuncIO {
    Arg(ArgNo),
    Field(FieldId),
    // Instructs the backend that it needs to create a temporary for this row and column.
    Temp(usize, usize),
}

impl From<ArgNo> for FuncIO {
    fn from(value: ArgNo) -> Self {
        Self::Arg(value)
    }
}

impl From<FieldId> for FuncIO {
    fn from(value: FieldId) -> Self {
        Self::Field(value)
    }
}

impl From<(usize, usize)> for FuncIO {
    fn from(value: (usize, usize)) -> Self {
        Self::Temp(value.0, value.1)
    }
}

impl TryInto<ArgNo> for FuncIO {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<ArgNo, Self::Error> {
        todo!()
    }
}

impl TryInto<FieldId> for FuncIO {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<FieldId, Self::Error> {
        todo!()
    }
}

impl TryInto<(usize, usize)> for FuncIO {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<(usize, usize), Self::Error> {
        todo!()
    }
}
