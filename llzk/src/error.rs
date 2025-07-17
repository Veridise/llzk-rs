use std::{
    convert::Infallible,
    error,
    fmt::{self, Display, Formatter},
    str::Utf8Error,
};

type MeliorError = melior::Error;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    BuildMthdFailed(&'static str),
    OutOfBoundsArgument(Option<String>, usize),
    OperationExpected(&'static str, String),
    Melior(MeliorError),
}

impl error::Error for Error {}

impl From<Utf8Error> for Error {
    fn from(error: Utf8Error) -> Self {
        Self::Melior(MeliorError::Utf8(error))
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl From<MeliorError> for Error {
    fn from(value: MeliorError) -> Self {
        Self::Melior(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::OperationExpected(op, actual) => write!(f, "{op} op expected: {actual}"),
            Error::Melior(error) => write!(f, "{error}"),
            Error::OutOfBoundsArgument(func_name, index) => {
                write!(f, "index {index} out of bounds ")?;
                match func_name {
                    Some(func_name) => {
                        write!(f, "function {func_name}")
                    }
                    None => write!(f, "block"),
                }
            }
            Error::BuildMthdFailed(mthd) => write!(f, "build method '{mthd}' failed"),
        }
    }
}
