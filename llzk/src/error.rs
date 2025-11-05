//! Types related to errors.

use std::{
    convert::Infallible,
    error,
    fmt::{self, Display, Formatter},
    str::Utf8Error,
};

type MeliorError = melior::Error;

/// Error type produced by the functions defined in this crate.
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    /// Happens when a custom operation factory function fails.
    BuildMthdFailed(&'static str),
    /// Happens when accessing an element in a collection by an index out of bounds.
    OutOfBoundsArgument(Option<String>, usize),
    /// Happens when attempting to cast a type erased operation into the wrong type.
    OperationExpected(&'static str, String),
    /// Happens when accessing a block in a region by an index out of bounds.
    BlockExpected(usize),
    /// Happens when attempting to get an operation from a block but the block is empty.
    EmptyBlock,
    /// Wrapper around [`melior::Error`] errors.
    Melior(MeliorError),
    /// Happens when an IR object doesn't have an attribute by that name.
    AttributeNotFound(String),
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
            Error::BlockExpected(nth) => {
                write!(
                    f,
                    "region was expected to have at least {} block{}",
                    nth + 1,
                    if *nth == 0 { "" } else { "s" }
                )
            }
            Error::EmptyBlock => write!(f, "block was expected not to be empty"),
            Error::AttributeNotFound(attr) => write!(f, "attribute was not found: {attr}"),
        }
    }
}
