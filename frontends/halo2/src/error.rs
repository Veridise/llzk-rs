use crate::halo2::Error;
use std::error::Error as StdError;
use std::io::Error as IOError;

/// Converts an implementation of [`std::error::Error`] into a [`crate::halo2::Error`].
pub fn to_plonk_error<E>(error: E) -> Error
where
    E: Into<Box<dyn StdError + Send + Sync>>,
{
    Error::Transcript(IOError::other(error))
}
