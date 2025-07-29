use std::fmt;

use regex::Regex;

use crate::display::{TextRepresentable, TextRepresentation};

lazy_static::lazy_static! {
    static ref INVALID_IDENT: Regex = Regex::new(r"[^A-Za-z0-9_]+").unwrap();
    pub(crate) static ref VALID_IDENT: Regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
}

const REPLACEMENT: &str = "_";

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Ident(String);

impl From<&str> for Ident {
    fn from(value: &str) -> Self {
        let replaced = INVALID_IDENT.replace_all(value, REPLACEMENT);
        Self(replaced.to_string())
    }
}

impl From<String> for Ident {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl Ident {
    pub fn value(&self) -> &String {
        &self.0
    }

    pub fn value_mut(&mut self) -> &mut String {
        &mut self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TextRepresentable for Ident {
    fn to_repr(&self) -> TextRepresentation {
        self.0.to_repr()
    }

    fn width_hint(&self) -> usize {
        self.0.len()
    }
}
