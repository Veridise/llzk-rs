use std::{
    convert::Infallible,
    error,
    fmt::{self, Display, Formatter},
    str::Utf8Error,
};

type MeliorError = melior::Error;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
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
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Error::OperationExpected(op, actual) => write!(formatter, "{op} op expected: {actual}"),
            Error::Melior(error) => write!(formatter, "{error}"),
        }
    }
}
