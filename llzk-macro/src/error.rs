//! Errors emitted by the macros

use std::borrow::Cow;

use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("Failed to strip prefix on pass {name} (tried prefixes: {prefixes:?})")]
    FailedToStripPassPrefix {
        prefixes: Vec<Cow<'static, str>>,
        name: String,
    },
}

impl Error {
    pub fn failed_to_strip<P>(
        prefixes: impl IntoIterator<Item = P>,
        name: &(impl ToString + ?Sized),
    ) -> Self
    where
        P: Into<Cow<'static, str>>,
    {
        Self::FailedToStripPassPrefix {
            prefixes: prefixes.into_iter().map(Into::into).collect(),
            name: name.to_string(),
        }
    }
}
