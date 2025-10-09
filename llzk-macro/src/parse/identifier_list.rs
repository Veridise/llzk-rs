//! List of identifiers. Based on [melior]'s.
//!
//! [melior]: https://github.com/mlir-rs/melior/blob/main/macro/src/parse/identifier_list.rs.

use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Result, Token,
};

/// Represents a comma-separated list of identifiers. Used as the DSL for the [`crate::conversion_passes`] macro.
pub struct IdentifierList {
    identifiers: Vec<Ident>,
}

impl IdentifierList {
    pub fn identifiers(&self) -> &[Ident] {
        &self.identifiers
    }
}

impl Parse for IdentifierList {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            identifiers: Punctuated::<Ident, Token![,]>::parse_terminated(input)?
                .into_iter()
                .collect(),
        })
    }
}
